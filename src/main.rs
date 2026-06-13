use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;

use openloop::cli::Cli;
use openloop::config;
use openloop::engine;
use openloop::tui;
use openloop::tui::TuiHandle;
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
        let goal_path = Path::new(&config.goal);
        if !goal_path.exists() {
            println!(
                "{} Goal file '{}' not found. Starting wizard to recreate.",
                "⚠".yellow(),
                config.goal
            );
            return wizard::run_wizard(&cli);
        }
        println!("{}", "Config loaded. Starting loop...".green());
        run_with_tui(&config, &cli.state_dir)
    } else {
        wizard::run_wizard(&cli)
    }
}

fn run_with_tui(config: &config::Config, state_dir: &str) -> Result<()> {
    let handle = Arc::new(TuiHandle::new(config.max_iterations));

    // Clone handle for engine thread
    let handle_clone = handle.clone();
    let config_clone = config.clone();
    let state_dir = state_dir.to_string();

    let engine_thread =
        std::thread::spawn(move || engine::run_loop_tui(&config_clone, &state_dir, &handle_clone));

    // Run TUI in main thread
    let tui_result = tui::run_tui(&handle);

    // Wait for engine
    let engine_result = engine_thread.join().unwrap_or_else(|e| {
        eprintln!("Engine thread panicked: {:?}", e);
        Ok(())
    });

    // Show engine error if any
    if let Err(e) = &engine_result {
        eprintln!("\n{} Engine error: {}", "✘".red(), e);
    }

    // TUI error
    if let Err(e) = tui_result {
        eprintln!("{} TUI error: {}", "⚠".yellow(), e);
    }

    engine_result
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
model = "openrouter/anthropic/claude-sonnet-4"
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
