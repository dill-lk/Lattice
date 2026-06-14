//! Legacy compatibility wrapper for `lattice-cli`.
//!
//! The official operator entrypoint is now the unified `lattice` binary.
//! This wrapper remains only for backwards compatibility.

use anyhow::Result;
use clap::Parser;
use lattice::cli::{self, CliCommands};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";

#[derive(Parser, Debug)]
#[command(name = "lattice-cli")]
#[command(version)]
#[command(about = "Compatibility wrapper for the unified `lattice` CLI")]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
struct LegacyCli {
    /// RPC endpoint URL
    #[arg(long, global = true, default_value = DEFAULT_RPC_URL)]
    rpc: String,

    #[command(subcommand)]
    command: CliCommands,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!(
        "[compat] `lattice-cli` is deprecated; prefer `lattice wallet|tx|query|status|peers ...` going forward."
    );

    let args = LegacyCli::parse();
    cli::run_cli(args.command, &args.rpc).await
}
