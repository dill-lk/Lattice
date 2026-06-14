//! Professional terminal formatter for the unified Lattice CLI.

use colored::*;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use lattice_core::tokenomics::{LATT_PER_LAT, TOKEN_SYMBOL};
use lattice_core::{Address, Amount, BlockHeight, Hash};
use num_format::{Locale, ToFormattedString};
use std::time::Duration;

const PANEL_WIDTH: usize = 72;

/// Print Lattice ASCII logo.
pub fn print_ascii_logo() {
    println!(
        "{}",
        "
    __         ______  ______  ______  __  ______  ______
   /\\ \\       /\\  __ \\/\\__  _\\/\\__  _\\/\\ \\/\\  ___\\/\\  ___\\
   \\ \\ \\____  \\ \\  __ \\/_/\\ \\/\\/_/\\ \\/\\ \\ \\ \\ \\___\\ \\  __\\
    \\ \\_____\\  \\ \\_\\ \\_\\ \\ \\_\\   \\ \\_\\ \\ \\_\\ \\_____\\ \\_____\\
     \\/_____/   \\/_/\\/_/  \\/_/    \\/_/  \\/_/\\/_____/\\/_____/
        "
        .cyan()
        .bold()
    );
}

/// Print the main CLI banner.
pub fn print_banner() {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    print_ascii_logo();
    println!(
        "  {}  {}",
        "LATTICE UNIFIED CLI".bold().cyan(),
        "v0.1.0".dimmed()
    );
    println!("  {}", "Post-quantum blockchain operator console".dimmed());
    println!();
}

/// Print a section title.
pub fn title(label: &str) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("{}", format!("\n▶ {}", label).bold().cyan());
}

/// Print a soft divider.
pub fn divider() {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("  {}", "─".repeat(PANEL_WIDTH).dimmed());
}

/// Print a compact status badge.
pub fn badge(label: &str, value: &str, color: &str) {
    let rendered = match color {
        "green" => value.green().bold(),
        "yellow" => value.yellow().bold(),
        "red" => value.red().bold(),
        "blue" => value.blue().bold(),
        _ => value.white().bold(),
    };
    println!("  {} {}", style(label).dim(), rendered);
}

/// Print a subtle note.
pub fn note(message: &str) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("  {} {}", "•".dimmed(), message.dimmed());
}

/// Format amount in LAT with proper decimals.
pub fn format_amount(amount: Amount) -> String {
    let whole = amount / LATT_PER_LAT;
    let frac = amount % LATT_PER_LAT;

    if frac == 0 {
        format!("{} {}", whole.to_formatted_string(&Locale::en), TOKEN_SYMBOL)
    } else {
        let frac_str = format!("{:08}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!(
            "{}.{} {}",
            whole.to_formatted_string(&Locale::en),
            trimmed,
            TOKEN_SYMBOL
        )
    }
}

/// Format amount with color based on size.
pub fn format_amount_colored(amount: Amount) -> ColoredString {
    let rendered = format_amount(amount);
    let lat = amount as f64 / LATT_PER_LAT as f64;

    if lat >= 1_000.0 {
        rendered.bright_green().bold()
    } else if lat >= 1.0 {
        rendered.green()
    } else if lat > 0.0 {
        rendered.yellow()
    } else {
        rendered.white()
    }
}

/// Format hash with ellipsis.
pub fn format_hash(hash: &Hash) -> String {
    let hex = hex::encode(hash);
    format!("0x{}…{}", &hex[..8], &hex[hex.len() - 8..])
}

/// Format full hash.
pub fn format_hash_full(hash: &Hash) -> String {
    format!("0x{}", hex::encode(hash))
}

/// Format address.
pub fn format_address(address: &Address) -> String {
    address.to_base58()
}

/// Format address with ellipsis.
pub fn format_address_short(address: &Address) -> String {
    let full = address.to_base58();
    if full.len() > 18 {
        format!("{}…{}", &full[..10], &full[full.len() - 8..])
    } else {
        full
    }
}

/// Format block height with separators.
pub fn format_height(height: BlockHeight) -> String {
    height.to_formatted_string(&Locale::en)
}

/// Format duration in human readable form.
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3_600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86_400 {
        format!("{}h {}m", secs / 3_600, (secs % 3_600) / 60)
    } else {
        format!("{}d {}h", secs / 86_400, (secs % 86_400) / 3_600)
    }
}

/// Print a success message.
pub fn success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print an error message.
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message.red());
}

/// Print a warning message.
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message.yellow());
}

/// Print an info message.
pub fn info(message: &str) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print a header.
pub fn header(title: &str) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("\n{}", title.bold().underline());
}

/// Print a subheader.
pub fn subheader(title: &str) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    println!("\n  {}", title.cyan().bold());
}

/// Print a key-value pair.
pub fn key_value(key: &str, value: &str) {
    println!("  {:<16} {}", style(key).dim(), value);
}

/// Print a key-value pair with colored value.
pub fn key_value_colored(key: &str, value: ColoredString) {
    println!("  {:<16} {}", style(key).dim(), value);
}

/// Print a simple table header.
pub fn table_header(columns: &[&str]) {
    if crate::cli::output::quiet_enabled() {
        return;
    }
    let header = columns
        .iter()
        .map(|column| format!("{:20}", column))
        .collect::<Vec<_>>()
        .join(" ");
    println!("\n{}", header.bold());
    println!("{}", "─".repeat(header.len()).dimmed());
}

/// Print a simple table row.
pub fn table_row(values: &[String]) {
    let row = values
        .iter()
        .map(|value| format!("{:20}", value))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{}", row);
}

/// Create a spinner.
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

/// Create a progress bar.
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Print a framed content box.
pub fn print_box(box_title: &str, content: &[String]) {
    title(box_title);
    divider();
    for line in content {
        println!("  {}", line);
    }
    println!();
}

/// Print a transaction summary card.
pub fn print_transaction_card(
    hash: &Hash,
    from: &Address,
    to: &Address,
    amount: Amount,
    status: &str,
    block: BlockHeight,
) {
    title("Transaction Details");
    divider();
    key_value("Hash", &format_hash_full(hash));
    key_value("From", &format_address(from));
    key_value("To", &format_address(to));
    key_value_colored("Amount", format_amount_colored(amount));

    let status_colored = match status {
        "Success" | "Confirmed" => status.green().bold(),
        "Failed" => status.red().bold(),
        _ => status.yellow().bold(),
    };
    key_value_colored("Status", status_colored);
    if block > 0 {
        key_value("Block", &format!("#{}", format_height(block)));
    }
    println!();
}

/// Print a wallet info card.
pub fn print_wallet_card(address: &Address, balance: Amount, nonce: u64) {
    title("Wallet Details");
    divider();
    key_value("Address", &format_address(address));
    key_value_colored("Balance", format_amount_colored(balance));
    key_value("Nonce", &nonce.to_string());
    println!();
}

/// Print a block info card.
pub fn print_block_card(
    height: BlockHeight,
    hash: &Hash,
    timestamp: u64,
    tx_count: usize,
    miner: &Address,
) {
    title(&format!("Block #{}", format_height(height)));
    divider();
    key_value("Hash", &format_hash_full(hash));
    key_value("Timestamp", &timestamp.to_string());
    key_value("Transactions", &tx_count.to_string());
    key_value("Miner", &format_address(miner));
    println!();
}

/// Print command help examples.
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
        assert_eq!(format_amount(LATT_PER_LAT), "1 LAT");
        assert_eq!(format_amount(LATT_PER_LAT + LATT_PER_LAT / 2), "1.5 LAT");
        assert_eq!(format_amount(1000 * LATT_PER_LAT), "1,000 LAT");
    }

    #[test]
    fn test_format_hash() {
        let hash = [1u8; 32];
        let formatted = format_hash(&hash);
        assert!(formatted.starts_with("0x"));
        assert!(formatted.contains('…'));
    }
}
