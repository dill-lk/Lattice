//! Network protocol and behavior for Lattice
//!
//! Defines message types and network behavior using libp2p.

use crate::error::NetworkError;
use borsh::{BorshDeserialize, BorshSerialize};
use lattice_core::{Block, BlockHeader, BlockHeight, Hash, Transaction};
use libp2p::{
    gossipsub::{
        Behaviour as Gossipsub, Config as GossipsubConfig, Event as GossipsubEvent, IdentTopic,
        MessageAuthenticity,
    },
    identity::Keypair,
    mdns::{self, tokio::Behaviour as Mdns, Event as MdnsEvent},
    request_response::{
        self, Behaviour as RequestResponse, Codec, Config as RequestResponseConfig,
        Event as RequestResponseEvent, Message as RRMessage, ProtocolSupport,
    },
    swarm::NetworkBehaviour,
    PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use std::{io, time::Duration};

/// Protocol version
pub const PROTOCOL_VERSION: &str = "/lattice/1.0.0";

/// Gossipsub topics
pub const TOPIC_BLOCKS: &str = "lattice/blocks";
pub const TOPIC_TRANSACTIONS: &str = "lattice/transactions";

/// Request-response protocol name
pub const SYNC_PROTOCOL: StreamProtocol = StreamProtocol::new("/lattice/sync/1.0.0");

/// Messages broadcast via gossipsub
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum GossipMessage {
    /// New block announcement
    NewBlock(Block),
    /// New transaction announcement
    NewTransaction(Transaction),
    /// Block header announcement (for header-first sync)
    NewBlockHeader(BlockHeader),
}

impl GossipMessage {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::error::Result<Vec<u8>> {
        borsh::to_vec(self).map_err(|e| NetworkError::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::error::Result<Self> {
        borsh::from_slice(bytes).map_err(|e| NetworkError::Serialization(e.to_string()))
    }

    /// Get topic for this message
    pub fn topic(&self) -> &str {
        match self {
            GossipMessage::NewBlock(_) | GossipMessage::NewBlockHeader(_) => TOPIC_BLOCKS,
            GossipMessage::NewTransaction(_) => TOPIC_TRANSACTIONS,
        }
    }
}

/// Request messages for request-response protocol
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum SyncRequest {
    /// Get current status (best height, genesis hash)
    GetStatus,
    /// Get block headers starting from a hash
    GetHeaders {
        start_hash: Hash,
        max_headers: u32,
    },
    /// Get full blocks by hash
    GetBlocks {
        hashes: Vec<Hash>,
    },
    /// Get transactions from mempool
    GetPooledTransactions {
        hashes: Vec<Hash>,
    },
}

/// Response messages for request-response protocol
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum SyncResponse {
    /// Status response
    Status {
        protocol_version: String,
        best_height: BlockHeight,
        best_hash: Hash,
        genesis_hash: Hash,
    },
    /// Headers response
    Headers(Vec<BlockHeader>),
    /// Blocks response
    Blocks(Vec<Block>),
    /// Pooled transactions response
    PooledTransactions(Vec<Transaction>),
    /// Error response
    Error(String),
}

impl SyncRequest {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::error::Result<Vec<u8>> {
        borsh::to_vec(self).map_err(|e| NetworkError::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::error::Result<Self> {
        borsh::from_slice(bytes).map_err(|e| NetworkError::Serialization(e.to_string()))
    }
}

impl SyncResponse {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::error::Result<Vec<u8>> {
        borsh::to_vec(self).map_err(|e| NetworkError::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::error::Result<Self> {
        borsh::from_slice(bytes).map_err(|e| NetworkError::Serialization(e.to_string()))
    }
}

/// Codec for sync protocol request-response
#[derive(Clone, Default)]
pub struct SyncCodec;

impl Codec for SyncCodec {
    type Protocol = StreamProtocol;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn read_request<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = io::Result<Self::Request>> + Send + 'async_trait>,
    >
    where
        T: libp2p::futures::AsyncRead + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            use libp2p::futures::AsyncReadExt;
            let mut len_bytes = [0u8; 4];
            io.read_exact(&mut len_bytes).await?;
            let len = u32::from_be_bytes(len_bytes) as usize;

            if len > 16 * 1024 * 1024 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
            }

            let mut buf = vec![0u8; len];
            io.read_exact(&mut buf).await?;
            Ok(buf)
        })
    }

    fn read_response<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = io::Result<Self::Response>> + Send + 'async_trait>,
    >
    where
        T: libp2p::futures::AsyncRead + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        self.read_request(protocol, io)
    }

    fn write_request<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
        req: Self::Request,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = io::Result<()>> + Send + 'async_trait>,
    >
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            use libp2p::futures::AsyncWriteExt;
            let len = (req.len() as u32).to_be_bytes();
            io.write_all(&len).await?;
            io.write_all(&req).await?;
            io.flush().await
        })
    }

    fn write_response<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
        res: Self::Response,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = io::Result<()>> + Send + 'async_trait>,
    >
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        self.write_request(protocol, io, res)
    }
}

/// Combined network behavior
#[derive(NetworkBehaviour)]
pub struct NetworkBehavior {
    /// Gossipsub for block and transaction propagation
    pub gossipsub: Gossipsub,
    /// mDNS for local peer discovery
    pub mdns: Mdns,
    /// Request-response for sync
    pub sync: RequestResponse<SyncCodec>,
}

/// Events from the network behavior
#[derive(Debug)]
pub enum NetworkEvent {
    /// New block received via gossip
    GossipBlock(Block),
    /// New transaction received via gossip
    GossipTransaction(Transaction),
    /// New block header received via gossip
    GossipBlockHeader(BlockHeader),
    /// Peer discovered via mDNS
    PeerDiscovered(PeerId),
    /// Peer expired from mDNS
    PeerExpired(PeerId),
    /// Sync request received
    SyncRequest {
        peer: PeerId,
        request_id: request_response::InboundRequestId,
        request: SyncRequest,
    },
    /// Sync response received
    SyncResponse {
        peer: PeerId,
        request_id: request_response::OutboundRequestId,
        response: SyncResponse,
    },
    /// Request failed
    SyncRequestFailed {
        peer: PeerId,
        request_id: request_response::OutboundRequestId,
        error: String,
    },
}

impl NetworkBehavior {
    /// Create a new network behavior
    pub fn new(keypair: &Keypair, _enable_mdns: bool) -> crate::error::Result<Self> {
        // Configure gossipsub
        let gossipsub_config = GossipsubConfig::default();

        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| NetworkError::Protocol(e.to_string()))?;

        // Configure mDNS
        let mdns = Mdns::new(mdns::Config::default(), keypair.public().to_peer_id())
            .map_err(|e| NetworkError::Protocol(e.to_string()))?;

        // Configure request-response
        let sync = RequestResponse::new(
            [(SYNC_PROTOCOL, ProtocolSupport::Full)],
            RequestResponseConfig::default()
                .with_request_timeout(Duration::from_secs(30))
                .with_max_concurrent_streams(100),
        );

        Ok(Self {
            gossipsub,
            mdns,
            sync,
        })
    }

    /// Subscribe to gossip topics
    pub fn subscribe_topics(&mut self) -> crate::error::Result<()> {
        let blocks_topic = IdentTopic::new(TOPIC_BLOCKS);
        let txs_topic = IdentTopic::new(TOPIC_TRANSACTIONS);

        self.gossipsub
            .subscribe(&blocks_topic)
            .map_err(|e| NetworkError::Protocol(format!("Failed to subscribe to blocks: {:?}", e)))?;

        self.gossipsub
            .subscribe(&txs_topic)
            .map_err(|e| NetworkError::Protocol(format!("Failed to subscribe to transactions: {:?}", e)))?;

        Ok(())
    }

    /// Publish a block to the network
    pub fn publish_block(&mut self, block: Block) -> crate::error::Result<()> {
        let msg = GossipMessage::NewBlock(block);
        let topic = IdentTopic::new(TOPIC_BLOCKS);
        let data = msg.to_bytes()?;

        self.gossipsub
            .publish(topic, data)
            .map_err(|e| NetworkError::Protocol(format!("Failed to publish block: {:?}", e)))?;

        Ok(())
    }

    /// Publish a transaction to the network
    pub fn publish_transaction(&mut self, tx: Transaction) -> crate::error::Result<()> {
        let msg = GossipMessage::NewTransaction(tx);
        let topic = IdentTopic::new(TOPIC_TRANSACTIONS);
        let data = msg.to_bytes()?;

        self.gossipsub
            .publish(topic, data)
            .map_err(|e| NetworkError::Protocol(format!("Failed to publish transaction: {:?}", e)))?;

        Ok(())
    }

    /// Send a sync request to a peer
    pub fn send_sync_request(
        &mut self,
        peer: &PeerId,
        request: SyncRequest,
    ) -> crate::error::Result<request_response::OutboundRequestId> {
        let data = request.to_bytes()?;
        Ok(self.sync.send_request(peer, data))
    }

    /// Send a sync response
    pub fn send_sync_response(
        &mut self,
        channel: request_response::ResponseChannel<Vec<u8>>,
        response: SyncResponse,
    ) -> crate::error::Result<()> {
        let data = response.to_bytes()?;
        self.sync
            .send_response(channel, data)
            .map_err(|_| NetworkError::Channel("Failed to send response".into()))
    }

    /// Process a gossipsub event and return network event
    pub fn process_gossipsub_event(&self, event: GossipsubEvent) -> Option<NetworkEvent> {
        match event {
            GossipsubEvent::Message {
                propagation_source: _,
                message_id: _,
                message,
            } => {
                match GossipMessage::from_bytes(&message.data) {
                    Ok(GossipMessage::NewBlock(block)) => Some(NetworkEvent::GossipBlock(block)),
                    Ok(GossipMessage::NewTransaction(tx)) => {
                        Some(NetworkEvent::GossipTransaction(tx))
                    }
                    Ok(GossipMessage::NewBlockHeader(header)) => {
                        Some(NetworkEvent::GossipBlockHeader(header))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to decode gossip message: {}", e);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// Process an mDNS event and return network event
    pub fn process_mdns_event(&self, event: MdnsEvent) -> Vec<NetworkEvent> {
        match event {
            MdnsEvent::Discovered(peers) => peers
                .into_iter()
                .map(|(peer_id, _)| NetworkEvent::PeerDiscovered(peer_id))
                .collect(),
            MdnsEvent::Expired(peers) => peers
                .into_iter()
                .map(|(peer_id, _)| NetworkEvent::PeerExpired(peer_id))
                .collect(),
        }
    }

    /// Process a request-response event and return network event
    pub fn process_sync_event(
        &self,
        event: RequestResponseEvent<Vec<u8>, Vec<u8>>,
    ) -> Option<(NetworkEvent, Option<request_response::ResponseChannel<Vec<u8>>>)> {
        match event {
            RequestResponseEvent::Message { peer, message } => match message {
                RRMessage::Request {
                    request_id,
                    request,
                    channel,
                } => match SyncRequest::from_bytes(&request) {
                    Ok(req) => Some((
                        NetworkEvent::SyncRequest {
                            peer,
                            request_id,
                            request: req,
                        },
                        Some(channel),
                    )),
                    Err(e) => {
                        tracing::warn!("Failed to decode sync request: {}", e);
                        None
                    }
                },
                RRMessage::Response {
                    request_id,
                    response,
                } => match SyncResponse::from_bytes(&response) {
                    Ok(resp) => Some((
                        NetworkEvent::SyncResponse {
                            peer,
                            request_id,
                            response: resp,
                        },
                        None,
                    )),
                    Err(e) => {
                        tracing::warn!("Failed to decode sync response: {}", e);
                        None
                    }
                },
            },
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => Some((
                NetworkEvent::SyncRequestFailed {
                    peer,
                    request_id,
                    error: format!("{:?}", error),
                },
                None,
            )),
            RequestResponseEvent::InboundFailure { peer, error, .. } => {
                tracing::warn!(?peer, ?error, "Inbound request failed");
                None
            }
            RequestResponseEvent::ResponseSent { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Address;

    #[test]
    fn test_gossip_message_serialization() {
        let block = Block::genesis();
        let msg = GossipMessage::NewBlock(block.clone());

        let bytes = msg.to_bytes().unwrap();
        let decoded = GossipMessage::from_bytes(&bytes).unwrap();

        match decoded {
            GossipMessage::NewBlock(b) => assert_eq!(b.hash(), block.hash()),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_sync_request_serialization() {
        let request = SyncRequest::GetHeaders {
            start_hash: [1u8; 32],
            max_headers: 100,
        };

        let bytes = request.to_bytes().unwrap();
        let decoded = SyncRequest::from_bytes(&bytes).unwrap();

        match decoded {
            SyncRequest::GetHeaders {
                start_hash,
                max_headers,
            } => {
                assert_eq!(start_hash, [1u8; 32]);
                assert_eq!(max_headers, 100);
            }
            _ => panic!("Wrong request type"),
        }
    }

    #[test]
    fn test_sync_response_serialization() {
        let response = SyncResponse::Status {
            protocol_version: PROTOCOL_VERSION.to_string(),
            best_height: 100,
            best_hash: [2u8; 32],
            genesis_hash: [0u8; 32],
        };

        let bytes = response.to_bytes().unwrap();
        let decoded = SyncResponse::from_bytes(&bytes).unwrap();

        match decoded {
            SyncResponse::Status {
                protocol_version,
                best_height,
                ..
            } => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
                assert_eq!(best_height, 100);
            }
            _ => panic!("Wrong response type"),
        }
    }
}
