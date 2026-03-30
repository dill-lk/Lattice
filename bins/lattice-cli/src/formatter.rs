//! Beautiful CLI formatter with Anthropic design patterns

use colored::*;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use lattice_core::{Address, Amount, BlockHeight, Hash};
use lattice_core::tokenomics::{LATT_PER_LAT, TOKEN_SYMBOL};
use num_format::{Locale, ToFormattedString};
use std::time::Duration;

/// Format amount in LAT with proper decimals (8 decimal places)
pub fn format_amount(amount: Amount) -> String {
    let lat = amount as f64 / LATT_PER_LAT as f64;
    if lat >= 1000.0 {
        format!("{:.2} {}", lat, TOKEN_SYMBOL)
    } else if lat >= 1.0 {
        format!("{:.4} {}", lat, TOKEN_SYMBOL)
    } else {
        format!("{:.8} {}", lat, TOKEN_SYMBOL)
    }
}

/// Format amount with color based on value
pub fn format_amount_colored(amount: Amount) -> ColoredString {
    let formatted = format_amount(amount);
    let lat = amount as f64 / LATT_PER_LAT as f64;
    
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

/// Print a box with content - simplified
pub fn print_box(title: &str, content: &[String]) {
    println!();
    println!("  {}", title.bold());
    println!("  {}", "─".repeat(50).dimmed());
    for line in content {
        println!("  {}", line);
    }
    println!();
}

/// Print a transaction card - minimal style
pub fn print_transaction_card(
    hash: &Hash,
    from: &Address,
    to: &Address,
    amount: Amount,
    status: &str,
    block: BlockHeight,
) {
    let status_colored = match status {
        "Success" => status.green(),
        "Failed" => status.red(),
        "Pending" => status.yellow(),
        _ => status.white(),
    };

    println!();
    println!("  {}", "Transaction".bold());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {}    {}", "Hash".dimmed(), format_hash(hash).white());
    println!("  {}    {}", "From".dimmed(), format_address_short(from));
    println!("  {}      {}", "To".dimmed(), format_address_short(to));
    println!("  {}  {}", "Amount".dimmed(), format_amount_colored(amount));
    println!("  {}  {}", "Status".dimmed(), status_colored);
    println!("  {}   #{}", "Block".dimmed(), format_height(block));
    println!();
}

/// Print a wallet info card - minimal style
pub fn print_wallet_card(address: &Address, balance: Amount, nonce: u64) {
    println!();
    println!("  {}", "Wallet".bold());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {} {}", "Address".dimmed(), format_address(address).white());
    println!("  {} {}", "Balance".dimmed(), format_amount_colored(balance));
    println!("  {}   {}", "Nonce".dimmed(), nonce.to_string().dimmed());
    println!();
}

/// Print a block info card - minimal style
pub fn print_block_card(
    height: BlockHeight,
    hash: &Hash,
    timestamp: u64,
    tx_count: usize,
    miner: &Address,
) {
    println!();
    println!("  {} {}", "Block".bold(), format!("#{}", format_height(height)).cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {}   {}", "Hash".dimmed(), format_hash(hash).white());
    println!("  {}   {}", "Time".dimmed(), timestamp.to_string().dimmed());
    println!("  {}    {}", "Txs".dimmed(), tx_count.to_string().white());
    println!("  {}  {}", "Miner".dimmed(), format_address_short(miner));
    println!();
}

/// Print welcome banner - clean and minimal
pub fn print_banner() {
    println!();
    println!(
        "  {}  {}",
        "LATTICE CLI".bold().cyan(),
        "v0.1.0".dimmed()
    );
    println!("  {}", "Quantum-Resistant Blockchain".dimmed());
    println!();
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
    use lattice_core::tokenomics::LATT_PER_LAT;

    #[test]
    fn test_format_amount() {
        // 1 LAT = 100_000_000 Latt (8 decimals)
        assert_eq!(format_amount(LATT_PER_LAT), "1.0000 LAT");
        assert_eq!(format_amount(LATT_PER_LAT + LATT_PER_LAT / 2), "1.5000 LAT");
        assert_eq!(format_amount(1000 * LATT_PER_LAT), "1000.00 LAT");
    }

    #[test]
    fn test_format_hash() {
        let hash = [1u8; 32];
        let formatted = format_hash(&hash);
        assert!(formatted.starts_with("0x"));
        assert!(formatted.contains("..."));
    }
}
