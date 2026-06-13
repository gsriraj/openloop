use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};

use crate::agent::discovery::{discover_agents, discover_models};
use crate::agent::runner::run_noninteractive;
use crate::agent::types::AgentConfig;
use crate::cli::Cli;
use crate::config;
use crate::engine;

pub fn run_wizard(cli: &Cli) -> Result<()> {
    println!("\n{}", "╭─────────────────────────────────────╮".cyan());
    println!(
        "{}",
        "│     OpenLoop Setup Wizard           │".cyan().bold()
    );
    println!("{}", "╰─────────────────────────────────────╯".cyan());
    println!();

    // Step 1: Detect agents
    let detected = discover_agents()?;

    if detected.is_empty() {
        eprintln!("{} No supported agent CLIs found on $PATH.", "⚠".yellow());
        eprintln!(
            "  Install one of: {}",
            "opencode, copilot, claude, aider, sweep".dimmed()
        );
        eprintln!("  Then re-run `openloop`.");
        return Ok(());
    }

    println!(
        "{} Detected agents: {}",
        "✓".green(),
        detected
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Step 2: Goal co-creation
    let goal_content = goal_co_creation(&detected[0])?;

    // Step 3: Agent selection
    let selected_agents = select_agents(&detected)?;

    // Step 4: Model configuration
    let (plan_model, verify_model, _plan_config) = select_models(&selected_agents)?;

    // Step 5: Execution mode
    let autopilot = select_execution_mode()?;

    // Step 6: Write config
    let config = build_config(&selected_agents, &plan_model, &verify_model, autopilot, cli)?;

    let state_dir = &cli.state_dir;
    std::fs::create_dir_all(state_dir)
        .with_context(|| format!("Failed to create {}", state_dir))?;
    config::save_config(&config, state_dir)?;
    println!(
        "  {} {}",
        "✔".green(),
        format!("{}/config.toml", state_dir).dimmed()
    );

    // Write GOAL.md
    let goal_path = Path::new(&config.goal);
    std::fs::write(goal_path, &goal_content)
        .with_context(|| format!("Failed to write {}", goal_path.display()))?;
    println!(
        "  {} {}",
        "✔".green(),
        goal_path.display().to_string().dimmed()
    );

    println!("\n{} Setup complete! Starting the loop...", "✓".green());

    // Step 7: Start the loop
    let config = config::load_config(cli)?;
    engine::run_loop(&config, &cli.state_dir)
}

fn goal_co_creation(agent: &AgentConfig) -> Result<String> {
    println!("\n{} Step 1: Define your goal", "──".bright_blue());

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let idea = Text::new("Describe your project in a sentence:")
        .with_help_message("e.g. \"A CLI to manage todo lists with deadlines\"")
        .prompt()?;

    let mut current_idea = idea.clone();

    println!(); // spacing before agent interaction

    // Try agent-guided goal refinement, fall back to manual if it fails
    let draft = try_agent_goal_coach(agent, &mut current_idea)
        .unwrap_or_else(|_| simple_goal_template(&current_idea));

    println!("\n{} Draft GOAL.md:\n", "──".bright_blue());
    for line in draft.lines().take(20) {
        println!("  {}", line);
    }
    if draft.lines().count() > 20 {
        println!(
            "  {}...",
            format!("({} more lines)", draft.lines().count() - 20).dimmed()
        );
    }

    let edit = Confirm::new("Edit the goal?")
        .with_default(true)
        .with_help_message(&format!("Opens {}", editor))
        .prompt()?;

    if edit {
        let tmpfile = format!("/tmp/openloop-goal-{}.md", std::process::id());
        std::fs::write(&tmpfile, &draft)?;

        let status = Command::new(&editor)
            .arg(&tmpfile)
            .status()
            .with_context(|| format!("Failed to open editor '{}'", editor))?;

        if status.success() {
            let edited = std::fs::read_to_string(&tmpfile)?;
            let _ = std::fs::remove_file(&tmpfile);
            return Ok(edited);
        }
        let _ = std::fs::remove_file(&tmpfile);
    }

    Ok(draft)
}

fn try_agent_goal_coach(agent: &AgentConfig, current_idea: &mut String) -> Result<String> {
    // Single round of clarifying questions (avoids repetition)
    let questions_prompt = format!(
        r#"You are a goal coach helping define a software project.

The user's rough idea:
"{current_idea}"

Ask 2-3 concise clarifying questions to help turn this into a well-defined project goal.
Questions should cover:
- Scope: what's in vs out of scope
- Success criteria: how will we know it's done?
- Platform/constraints: any technical requirements?

Output ONLY the questions, one per line, with no preamble or numbering."#,
    );

    print!(
        "  {} Asking agent for clarifying questions...",
        "⏳".yellow()
    );
    std::io::stdout().flush().ok();
    let questions_result = run_interactive(agent, &questions_prompt)?;
    print!(
        "\r  {} Agent responded                        \n",
        "✓".green()
    );
    std::io::stdout().flush().ok();

    let questions: Vec<&str> = questions_result
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && t.trim_end_matches(|c: char| c.is_ascii_punctuation()).len() > 5
        })
        .collect();

    if !questions.is_empty() {
        println!("\n{}", "─".repeat(40).dimmed());
        println!("{}", "The agent asked these clarifying questions:".cyan());
        println!();

        for q in &questions {
            println!("  {}", q.trim().yellow());
        }

        println!();
        let answers = Text::new("Your answers:")
            .with_help_message("Be specific — this shapes the final goal")
            .prompt()?;

        *current_idea = format!("{}\n\nClarifications:\n{}", *current_idea, answers);
    }

    print!("  {} Generating structured goal...", "⏳".yellow());
    std::io::stdout().flush().ok();

    let generate_prompt = format!(
        "You are a goal coach. Based on the following project description, produce a GOAL.md file.\n\
         The goal should have:\n\
         1. A clear project description\n\
         2. 5-10 specific, testable success criteria (as a checklist)\n\
         3. Out of scope items (what won't be built)\n\
         4. Technical constraints if applicable\n\
         \n\
         Project description:\n\
         {current_idea}\n\
         \n\
         Output ONLY the GOAL.md content, starting with a level-1 heading 'Project Goal'."
    );

    let draft = run_interactive(agent, &generate_prompt)?;
    print!(
        "\r  {} Goal drafted                              \n",
        "✓".green()
    );
    std::io::stdout().flush().ok();

    Ok(draft)
}

fn simple_goal_template(idea: &str) -> String {
    format!(
        "# Project Goal\n\
         \n\
         {idea}\n\
         \n\
         ## Success Criteria\n\
         \n\
         - [ ] Define clear success criteria\n\
         - [ ] Implement core functionality\n\
         - [ ] All tests pass\n\
         \n\
         ## Out of Scope\n\
         \n\
         - TBD\n"
    )
}

fn select_agents(detected: &[AgentConfig]) -> Result<Vec<AgentConfig>> {
    println!("\n{} Step 2: Select agent CLIs", "──".bright_blue());

    if detected.len() == 1 {
        println!(
            "  {} {} (only one detected, auto-selected)",
            "✓".green(),
            detected[0].name
        );
        return Ok(vec![detected[0].clone()]);
    }

    let options: Vec<&str> = detected.iter().map(|a| a.name.as_str()).collect();
    let selected = MultiSelect::new("Which agents should execute tasks?", options)
        .with_help_message("Space to select, Enter to confirm")
        .with_default(&[0])
        .prompt()?;

    Ok(detected
        .iter()
        .filter(|a| selected.contains(&a.name.as_str()))
        .cloned()
        .collect())
}

fn select_models(agents: &[AgentConfig]) -> Result<(String, String, HashMap<String, String>)> {
    println!("\n{} Step 3: Model configuration", "──".bright_blue());

    let primary_agent = agents
        .first()
        .map(|a| &a.name)
        .map(|s| s.as_str())
        .unwrap_or("opencode");

    print!(
        "  {} Fetching available models from {}...",
        "⏳".yellow(),
        primary_agent
    );
    std::io::stdout().flush().ok();
    let models = discover_models(primary_agent).unwrap_or_else(|_| fallback_models(primary_agent));
    print!(
        "\r  {} {} models available               \n",
        "✓".green(),
        models.len()
    );

    let plan_model = Select::new("Planning model:", models.clone())
        .with_starting_cursor(0)
        .with_help_message("Used for planning steps and executing code")
        .prompt()?;

    let same = Confirm::new("Same model for goal verification?")
        .with_default(true)
        .with_help_message("A separate model can catch mistakes the planner missed")
        .prompt()?;

    let verify_model = if same {
        plan_model.to_string()
    } else {
        let verify_cursor = 1.min(models.len().saturating_sub(1));
        Select::new("Verification model:", models)
            .with_starting_cursor(verify_cursor)
            .with_help_message("Used to check if success criteria are met")
            .prompt()?
            .to_string()
    };

    Ok((plan_model.to_string(), verify_model, HashMap::new()))
}

fn fallback_models(agent: &str) -> Vec<String> {
    match agent {
        "opencode" => vec![
            "openrouter/anthropic/claude-sonnet-4".into(),
            "openrouter/anthropic/claude-haiku-4.5".into(),
            "openrouter/anthropic/claude-opus-4".into(),
            "openrouter/openai/gpt-4o".into(),
            "openrouter/google/gemini-flash-latest".into(),
        ],
        "copilot" => vec![
            "gpt-4o".into(),
            "gpt-4o-mini".into(),
            "claude-sonnet-4-20250514".into(),
        ],
        _ => vec!["default".into()],
    }
}

fn select_execution_mode() -> Result<bool> {
    println!("\n{} Step 4: Execution mode", "──".bright_blue());

    let options = vec!["Step-by-step (recommended for first run)", "Full autopilot"];

    let selection = Select::new("How should the loop run?", options)
        .with_starting_cursor(0)
        .prompt()?;

    Ok(selection.starts_with("Full"))
}

fn build_config(
    agents: &[AgentConfig],
    plan_model: &str,
    verify_model: &str,
    autopilot: bool,
    _cli: &Cli,
) -> Result<config::Config> {
    let mut agent_configs = HashMap::new();

    for agent in agents {
        let mut model_config = HashMap::new();
        model_config.insert("verify_model".to_string(), verify_model.to_string());

        agent_configs.insert(
            agent.name.clone(),
            config::AgentConfig {
                model: plan_model.to_string(),
                model_config,
            },
        );
    }

    Ok(config::Config {
        goal: "GOAL.md".to_string(),
        max_iterations: 50,
        autopilot,
        parallel: false,
        agents: config::AgentsSection {
            enabled: agents.iter().map(|a| a.name.clone()).collect(),
            configs: agent_configs,
        },
        state: config::StateConfig {
            file: "state.md".to_string(),
        },
    })
}

fn run_interactive(agent: &AgentConfig, prompt: &str) -> Result<String> {
    run_noninteractive(agent, prompt)
}
