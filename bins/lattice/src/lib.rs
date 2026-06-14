//! Shared library surface for the unified `lattice` binary.
//!
//! This exposes the all-in-one command modules so the repository can standardise
//! on a single user-facing executable while still reusing the same code across
//! docs, install scripts, and any legacy wrappers.

pub mod cli;
pub mod miner;
pub mod node;

/// Professional help footer shown by the unified CLI.
pub const CLI_AFTER_HELP: &str = r#"Examples:
  lattice
  lattice --wallet-new
  lattice --balance <ADDRESS>
  lattice --send <ADDRESS> --amount 1.5
  lattice --node
  lattice --mine 4
  lattice wallet create
  lattice query block latest
  lattice doctor
  lattice --json status
  lattice completion bash
  lattice config init --path node.toml

Notes:
  • `lattice` is the official all-in-one executable.
  • Legacy binaries such as `lattice-cli`, `lattice-node`, and `lattice-miner`
    are compatibility paths only.
  • If `lattice miner ...` targets the default local RPC and no node is running,
    Lattice can fall back to integrated local miner-node mode.
  • Amounts are interpreted as LAT by default. Use `--latt` in advanced tx paths
    for raw base units.
"#;
