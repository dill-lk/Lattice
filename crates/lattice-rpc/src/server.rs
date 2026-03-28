//! JSON-RPC 2.0 server implementation

use crate::error::RpcError;
use crate::handlers::RpcHandlers;
use crate::types::{RpcRequest, RpcResponse};
use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

/// RPC server configuration
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Enable CORS
    pub cors_enabled: bool,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8545,
            cors_enabled: true,
        }
    }
}

impl RpcConfig {
    /// Create config with custom host and port
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            cors_enabled: true,
        }
    }

    /// Get the socket address
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }
}

/// Shared server state
struct AppState {
    handlers: RpcHandlers,
}

/// JSON-RPC 2.0 server
pub struct RpcServer {
    config: RpcConfig,
    handlers: RpcHandlers,
}

impl RpcServer {
    /// Create a new RPC server with default handlers
    pub fn new(config: RpcConfig) -> Self {
        Self {
            config,
            handlers: RpcHandlers::new(),
        }
    }

    /// Create RPC server with custom handlers
    pub fn with_handlers(config: RpcConfig, handlers: RpcHandlers) -> Self {
        Self { config, handlers }
    }

    /// Build the axum router
    fn build_router(self) -> Router {
        let state = Arc::new(AppState {
            handlers: self.handlers,
        });

        let mut app = Router::new()
            .route("/", post(handle_rpc))
            .with_state(state);

        if self.config.cors_enabled {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::POST, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

            app = app.layer(cors);
        }

        app
    }

    /// Run the server
    pub async fn run(self) -> Result<(), crate::error::Error> {
        let addr = self.config.socket_addr();
        info!("Starting RPC server on {}", addr);

        let router = self.build_router();

        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(listener, router)
            .await
            .map_err(|e| crate::error::Error::Server(e.to_string()))?;

        Ok(())
    }

    /// Run the server with graceful shutdown
    pub async fn run_until_stopped(
        self,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), crate::error::Error> {
        let addr = self.config.socket_addr();
        info!("Starting RPC server on {}", addr);

        let router = self.build_router();

        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| crate::error::Error::Server(e.to_string()))?;

        info!("RPC server shutdown complete");
        Ok(())
    }
}

/// Handle incoming RPC requests
async fn handle_rpc(
    State(state): State<Arc<AppState>>,
    body: String,
) -> impl IntoResponse {
    // Try to parse as batch request first
    if body.trim_start().starts_with('[') {
        return handle_batch_request(&state, &body).await;
    }

    // Single request
    match serde_json::from_str::<RpcRequest>(&body) {
        Ok(request) => {
            let response = process_request(&state.handlers, request);
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!("Failed to parse RPC request: {}", e);
            let response = RpcResponse::error(serde_json::Value::Null, RpcError::parse_error());
            (StatusCode::OK, Json(response))
        }
    }
}

/// Handle batch RPC requests
async fn handle_batch_request(
    state: &AppState,
    body: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    match serde_json::from_str::<Vec<RpcRequest>>(body) {
        Ok(requests) => {
            if requests.is_empty() {
                let response = RpcResponse::error(
                    serde_json::Value::Null,
                    RpcError::invalid_request(),
                );
                return (StatusCode::OK, Json(serde_json::to_value(response).unwrap()));
            }

            let responses: Vec<RpcResponse> = requests
                .into_iter()
                .map(|req| process_request(&state.handlers, req))
                .collect();

            (StatusCode::OK, Json(serde_json::to_value(responses).unwrap()))
        }
        Err(e) => {
            error!("Failed to parse batch RPC request: {}", e);
            let response = RpcResponse::error(serde_json::Value::Null, RpcError::parse_error());
            (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
        }
    }
}

/// Process a single RPC request
fn process_request(handlers: &RpcHandlers, request: RpcRequest) -> RpcResponse {
    // Validate JSON-RPC version
    if !request.validate() {
        return RpcResponse::error(request.id, RpcError::invalid_request());
    }

    // Execute the method
    match handlers.handle(&request.method, request.params) {
        Ok(result) => RpcResponse::success(request.id, result),
        Err(error) => RpcResponse::error(request.id, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RpcConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8545);
        assert!(config.cors_enabled);
    }

    #[test]
    fn test_config_socket_addr() {
        let config = RpcConfig::new("0.0.0.0", 9000);
        let addr = config.socket_addr();
        assert_eq!(addr.port(), 9000);
    }
}
