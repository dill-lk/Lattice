//! Output mode helpers for human vs JSON rendering.

use serde_json::Value;

const ENV_OUTPUT_MODE: &str = "LATTICE_OUTPUT_MODE";
const ENV_QUIET: &str = "LATTICE_QUIET";
const ENV_VERBOSE: &str = "LATTICE_VERBOSE";

pub fn configure(json: bool, quiet: bool, verbose: bool) {
    std::env::set_var(ENV_OUTPUT_MODE, if json { "json" } else { "human" });
    std::env::set_var(ENV_QUIET, if quiet { "1" } else { "0" });
    std::env::set_var(ENV_VERBOSE, if verbose { "1" } else { "0" });
}

pub fn json_enabled() -> bool {
    std::env::var(ENV_OUTPUT_MODE)
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

pub fn quiet_enabled() -> bool {
    std::env::var(ENV_QUIET).map(|v| v == "1").unwrap_or(false)
}

pub fn verbose_enabled() -> bool {
    std::env::var(ENV_VERBOSE).map(|v| v == "1").unwrap_or(false)
}

pub fn human_output_enabled() -> bool {
    !json_enabled()
}

pub fn print_banner_if_needed() {
    if human_output_enabled() && !quiet_enabled() {
        crate::cli::formatter::print_banner();
    }
}

pub fn emit_json(value: Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
