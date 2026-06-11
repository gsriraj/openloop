use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;

use openloop::cli::Cli;
use openloop::config;
use openloop::engine;
use openloop::wizard;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.init {
        return cmd_init(&cli);
    }

    if cli.status {
        return cmd_status(&cli);
    }

    if cli.is_headless() {
        return run_headless(&cli);
    }

    let config_path = Path::new(&cli.state_dir).join("config.toml");
    if config_path.exists() {
        let config = config::load_config(&cli)?;
        println!("{}", "Config loaded. Starting loop...".green());
        engine::run_loop(&config, &cli.state_dir)
    } else {
        wizard::run_wizard(&cli)
    }
}

fn cmd_init(cli: &Cli) -> Result<()> {
    let dir = Path::new(&cli.state_dir);
    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", cli.state_dir))?;

    let config_path = dir.join("config.toml");
    if !config_path.exists() {
        let default_config = r#"goal = "GOAL.md"
max_iterations = 50
autopilot = false
parallel = false

[agents]
enabled = ["opencode"]

[agents.opencode]
model = "claude-sonnet-4-20250514"
model_config = { temperature = 0.7, max_tokens = 8192 }

[state]
file = "state.md"
"#;
        std::fs::write(&config_path, default_config)
            .with_context(|| format!("Failed to write {}", config_path.display()))?;
        println!("  {} {}", "✔".green(), config_path.display());
    } else {
        println!(
            "  {} {} (already exists)",
            "•".yellow(),
            config_path.display()
        );
    }

    let goal_path = Path::new("GOAL.md");
    if !goal_path.exists() {
        let example_goal = r#"# Project Goal

Build a CLI tool that ...

## Success Criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] All tests pass
"#;
        std::fs::write(goal_path, example_goal)
            .with_context(|| format!("Failed to write {}", goal_path.display()))?;
        println!("  {} {}", "✔".green(), goal_path.display());
    } else {
        println!(
            "  {} {} (already exists)",
            "•".yellow(),
            goal_path.display()
        );
    }

    println!(
        "\n{} Initialized {}. Run `openloop` to start the loop.",
        "Done.".green(),
        cli.state_dir
    );
    Ok(())
}

fn cmd_status(cli: &Cli) -> Result<()> {
    let state_path = Path::new(&cli.state_dir).join("state.md");
    if state_path.exists() {
        let content = std::fs::read_to_string(&state_path)
            .with_context(|| format!("Failed to read {}", state_path.display()))?;
        println!("{}", content);
    } else {
        println!("No state file found at {}", state_path.display());
    }
    Ok(())
}

#[allow(dead_code)]
fn run_headless(cli: &Cli) -> Result<()> {
    let config = config::load_config(cli)?;
    engine::run_loop(&config, &cli.state_dir)
}
