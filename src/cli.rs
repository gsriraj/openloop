use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "openloop",
    about = "Loop engineering — delegate work to coding agents until a goal is achieved",
    version,
    after_help = "Run without arguments to launch the interactive setup wizard."
)]
pub struct Cli {
    #[arg(long, help = "Run fully autonomously (no human confirmations)")]
    pub autopilot: bool,

    #[arg(long, help = "Scaffold .openloop/ config directory + example GOAL.md")]
    pub init: bool,

    #[arg(long, help = "Display current loop state and progress")]
    pub status: bool,

    #[arg(long, help = "Allow parallel agent execution (uses git worktrees)")]
    pub parallel: bool,

    #[arg(
        long,
        default_value = "GOAL.md",
        help = "Goal file path (default: ./GOAL.md)"
    )]
    pub goal: String,

    #[arg(
        long = "agent-cli",
        value_name = "NAME",
        help = "Agent CLI(s) to use (repeatable, e.g. opencode copilot)"
    )]
    pub agent_cli: Vec<String>,

    #[arg(long, help = "Model override for all agents")]
    pub model: Option<String>,

    #[arg(
        long = "model-config",
        value_name = "KEY=VAL",
        help = "Model params (repeatable, e.g. temperature=0.7)"
    )]
    pub model_config: Vec<String>,

    #[arg(long, default_value = "50", help = "Max loop iterations (default: 50)")]
    pub max_iterations: u32,

    #[arg(
        long,
        default_value = ".openloop",
        help = "Config/state directory (default: .openloop)"
    )]
    pub state_dir: String,
}

impl Cli {
    pub fn is_headless(&self) -> bool {
        self.init
            || self.status
            || !self.agent_cli.is_empty()
            || self.goal != "GOAL.md"
            || self.autopilot
    }
}
