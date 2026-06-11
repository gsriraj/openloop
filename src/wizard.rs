use anyhow::Result;
use colored::Colorize;

use crate::cli::Cli;

pub fn run_wizard(_cli: &Cli) -> Result<()> {
    println!(
        "{}",
        "Interactive setup wizard — not yet implemented.".yellow()
    );
    Ok(())
}