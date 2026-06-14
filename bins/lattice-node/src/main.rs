//! Legacy compatibility wrapper for `lattice-node`.
//!
//! The official operator entrypoint is now the unified `lattice` binary.
//! This wrapper remains only for backwards compatibility.

use anyhow::Result;
use clap::Parser;
use lattice::node::{self, NodeArgs};

#[derive(Parser, Debug)]
#[command(name = "lattice-node")]
#[command(version)]
#[command(about = "Compatibility wrapper for the unified `lattice node` command")]
struct LegacyNodeCli {
    #[command(flatten)]
    node: NodeArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!(
        "[compat] `lattice-node` is deprecated; prefer `lattice node ...` going forward."
    );

    let args = LegacyNodeCli::parse();
    node::run_node(args.node).await
}
