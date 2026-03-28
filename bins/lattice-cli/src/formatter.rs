//! Beautiful CLI formatter with Anthropic design patterns

use colored::*;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use lattice_core::{Address, Amount, BlockHeight, Hash};
use num_format::{Locale, ToFormattedString};
use std::time::Duration;

/// Format amount in LAT with proper decimals
pub fn format_amount(amount: Amount) -> String {
    let lat = amount as f64 / 1_000_000_000_000_000_000.0;
    if lat >= 1000.0 {
        format!("{:.2} LAT", lat)
    } else if lat >= 1.0 {
        format!("{:.4} LAT", lat)
    } else {
        format!("{:.8} LAT", lat)
    }
}

/// Format amount with color based on value
pub fn format_amount_colored(amount: Amount) -> ColoredString {
    let formatted = format_amount(amount);
    let lat = amount as f64 / 1_000_000_000_000_000_000.0;
    
    if lat >= 1000.0 {
        formatted.bright_green().bold()
    } else if lat >= 100.0 {
        formatted.green()
    } else if lat >= 1.0 {
        formatted.yellow()
    } else {
        formatted.white()
    }
}

/// Format hash with ellipsis
pub fn format_hash(hash: &Hash) -> String {
    let hex = hex::encode(hash);
    format!("0x{}...{}", &hex[..8], &hex[hex.len()-8..])
}

/// Format full hash
pub fn format_hash_full(hash: &Hash) -> String {
    format!("0x{}", hex::encode(hash))
}

/// Format address with checksum
pub fn format_address(address: &Address) -> String {
    address.to_base58()
}

/// Format address with ellipsis
pub fn format_address_short(address: &Address) -> String {
    let full = address.to_base58();
    if full.len() > 16 {
        format!("{}...{}", &full[..8], &full[full.len()-8..])
    } else {
        full
    }
}

/// Format block height with thousands separator
pub fn format_height(height: BlockHeight) -> String {
    (height as u64).to_formatted_string(&Locale::en)
}

/// Format duration in human readable form
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message.red());
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message.yellow());
}

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print a header
pub fn header(title: &str) {
    println!("\n{}", title.bold().underline());
}

/// Print a subheader
pub fn subheader(title: &str) {
    println!("\n  {}", title.cyan().bold());
}

/// Print a key-value pair
pub fn key_value(key: &str, value: &str) {
    println!("  {}: {}", style(key).dim(), value);
}

/// Print a key-value pair with colored value
pub fn key_value_colored(key: &str, value: ColoredString) {
    println!("  {}: {}", style(key).dim(), value);
}

/// Print a table header
pub fn table_header(columns: &[&str]) {
    let header = columns
        .iter()
        .map(|c| format!("{:20}", c))
        .collect::<Vec<_>>()
        .join(" ");
    println!("\n{}", header.bold());
    println!("{}", "─".repeat(header.len()).dimmed());
}

/// Print a table row
pub fn table_row(values: &[String]) {
    let row = values
        .iter()
        .map(|v| format!("{:20}", v))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{}", row);
}

/// Create a spinner with message
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Print a box with content
pub fn print_box(title: &str, content: &[String]) {
    let max_width = content.iter().map(|s| s.len()).max().unwrap_or(0).max(title.len());
    let width = max_width + 4;

    // Top border
    println!("╭{}╮", "─".repeat(width));
    
    // Title
    let padding = (width - title.len()) / 2;
    println!("│{}{}{:padding$}│", " ".repeat(padding), title.bold(), "", padding = width - padding - title.len());
    println!("├{}┤", "─".repeat(width));
    
    // Content
    for line in content {
        println!("│  {:width$}  │", line, width = max_width);
    }
    
    // Bottom border
    println!("╰{}╯", "─".repeat(width));
}

/// Print a transaction card
pub fn print_transaction_card(
    hash: &Hash,
    from: &Address,
    to: &Address,
    amount: Amount,
    status: &str,
    block: BlockHeight,
) {
    println!("\n╭{}╮", "─".repeat(68));
    println!("│  {}  │", "Transaction Details".bold().cyan());
    println!("├{}┤", "─".repeat(68));
    println!("│  {:18} {}  │", "Hash:", format_hash(hash).bright_white());
    println!("│  {:18} {}  │", "From:", format_address_short(from).white());
    println!("│  {:18} {}  │", "To:", format_address_short(to).white());
    println!("│  {:18} {}  │", "Amount:", format_amount_colored(amount));
    
    let status_colored = match status {
        "Success" => status.green().bold(),
        "Failed" => status.red().bold(),
        "Pending" => status.yellow().bold(),
        _ => status.white(),
    };
    println!("│  {:18} {}  │", "Status:", status_colored);
    println!("│  {:18} {}  │", "Block:", format_height(block).bright_white());
    println!("╰{}╯", "─".repeat(68));
}

/// Print a wallet info card
pub fn print_wallet_card(address: &Address, balance: Amount, nonce: u64) {
    println!("\n╭{}╮", "─".repeat(68));
    println!("│  {}  │", "Wallet Information".bold().cyan());
    println!("├{}┤", "─".repeat(68));
    println!("│  {:18} {}  │", "Address:", format_address(address).bright_white());
    println!("│  {:18} {}  │", "Balance:", format_amount_colored(balance));
    println!("│  {:18} {}  │", "Nonce:", nonce.to_string().white());
    println!("╰{}╯", "─".repeat(68));
}

/// Print a block info card
pub fn print_block_card(
    height: BlockHeight,
    hash: &Hash,
    timestamp: u64,
    tx_count: usize,
    miner: &Address,
) {
    println!("\n╭{}╮", "─".repeat(68));
    println!("│  {}  │", format!("Block #{}", format_height(height)).bold().cyan());
    println!("├{}┤", "─".repeat(68));
    println!("│  {:18} {}  │", "Hash:", format_hash(hash).bright_white());
    println!("│  {:18} {}  │", "Timestamp:", timestamp.to_string().white());
    println!("│  {:18} {}  │", "Transactions:", tx_count.to_string().bright_white());
    println!("│  {:18} {}  │", "Miner:", format_address_short(miner).white());
    println!("╰{}╯", "─".repeat(68));
}

/// Print welcome banner
pub fn print_banner() {
    let banner = r#"
    ╔══════════════════════════════════════════════════════════╗
    ║                                                          ║
    ║   ██╗      █████╗ ████████╗████████╗██╗ ██████╗███████╗ ║
    ║   ██║     ██╔══██╗╚══██╔══╝╚══██╔══╝██║██╔════╝██╔════╝ ║
    ║   ██║     ███████║   ██║      ██║   ██║██║     █████╗   ║
    ║   ██║     ██╔══██║   ██║      ██║   ██║██║     ██╔══╝   ║
    ║   ███████╗██║  ██║   ██║      ██║   ██║╚██████╗███████╗ ║
    ║   ╚══════╝╚═╝  ╚═╝   ╚═╝      ╚═╝   ╚═╝ ╚═════╝╚══════╝ ║
    ║                                                          ║
    ║          Quantum-Resistant Blockchain                   ║
    ║                  CLI v0.1.0                             ║
    ║                                                          ║
    ╚══════════════════════════════════════════════════════════╝
    "#;

    println!("{}", banner.bright_cyan().bold());
}

/// Print command help with nice formatting
pub fn print_command_help(command: &str, description: &str, examples: &[(&str, &str)]) {
    println!("\n{}", command.bold().cyan());
    println!("  {}", description);
    
    if !examples.is_empty() {
        println!("\n  {}", "Examples:".bold());
        for (cmd, desc) in examples {
            println!("    {} {}", "$".dimmed(), cmd.bright_white());
            println!("      {}", desc.dimmed());
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(1_000_000_000_000_000_000), "1.0000 LAT");
        assert_eq!(format_amount(1_500_000_000_000_000_000), "1.5000 LAT");
        assert_eq!(format_amount(1000_000_000_000_000_000_000), "1000.00 LAT");
    }

    #[test]
    fn test_format_hash() {
        let hash = [1u8; 32];
        let formatted = format_hash(&hash);
        assert!(formatted.starts_with("0x"));
        assert!(formatted.contains("..."));
    }
}
