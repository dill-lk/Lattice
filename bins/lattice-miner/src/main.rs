//! Legacy compatibility wrapper for `lattice-miner`.
//!
//! The official operator entrypoint is now the unified `lattice` binary.
//! This wrapper remains only for backwards compatibility.

use anyhow::Result;
use clap::Parser;
use lattice::miner::{self, MinerArgs};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";

#[derive(Parser, Debug)]
#[command(name = "lattice-miner")]
#[command(version)]
#[command(about = "Compatibility wrapper for the unified `lattice miner` command")]
struct LegacyMinerCli {
    /// RPC endpoint URL to connect to
    #[arg(long, short, default_value = DEFAULT_RPC_URL)]
    rpc: String,

    #[command(flatten)]
    miner: MinerArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!(
        "[compat] `lattice-miner` is deprecated; prefer `lattice miner ...` going forward."
    );

    let args = LegacyMinerCli::parse();
    miner::run_miner(args.miner, &args.rpc).await
}
