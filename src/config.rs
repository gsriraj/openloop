use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    #[serde(default)]
    pub model_config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    #[serde(default = "default_state_file")]
    pub file: String,
}

fn default_state_file() -> String {
    "state.md".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_goal")]
    pub goal: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default)]
    pub autopilot: bool,
    #[serde(default)]
    pub parallel: bool,
    #[serde(default)]
    pub agents: AgentsSection,
    #[serde(default)]
    pub state: StateConfig,
}

fn default_goal() -> String {
    "GOAL.md".to_string()
}

fn default_max_iterations() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsSection {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(flatten)]
    pub configs: HashMap<String, AgentConfig>,
}

impl Default for AgentsSection {
    fn default() -> Self {
        let mut configs = HashMap::new();
        configs.insert(
            "opencode".to_string(),
            AgentConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                model_config: HashMap::new(),
            },
        );
        AgentsSection {
            enabled: vec!["opencode".to_string()],
            configs,
        }
    }
}

impl Default for StateConfig {
    fn default() -> Self {
        StateConfig {
            file: "state.md".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            goal: default_goal(),
            max_iterations: default_max_iterations(),
            autopilot: false,
            parallel: false,
            agents: AgentsSection::default(),
            state: StateConfig::default(),
        }
    }
}

#[allow(dead_code)]
impl Config {
    pub fn state_path(&self, state_dir: &str) -> String {
        format!("{}/{}", state_dir, self.state.file)
    }
}

#[allow(dead_code)]
pub fn save_config(config: &Config, state_dir: &str) -> Result<()> {
    let config_path = Path::new(state_dir).join("config.toml");
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, contents)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;
    Ok(())
}

pub fn load_config(cli: &Cli) -> Result<Config> {
    let config_path = Path::new(&cli.state_dir).join("config.toml");
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    let mut config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;

    merge_cli_flags(&mut config, cli);
    Ok(config)
}

fn merge_cli_flags(config: &mut Config, cli: &Cli) {
    if cli.autopilot {
        config.autopilot = true;
    }
    if cli.parallel {
        config.parallel = true;
    }
    if !cli.agent_cli.is_empty() {
        config.agents.enabled = cli.agent_cli.clone();
    }
    if let Some(ref model) = cli.model {
        for name in &config.agents.enabled {
            if let Some(agent) = config.agents.configs.get_mut(name) {
                agent.model = model.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.goal, "GOAL.md");
        assert_eq!(config.max_iterations, 50);
        assert!(!config.autopilot);
        assert!(!config.parallel);
        assert!(config.agents.enabled.contains(&"opencode".to_string()));
    }

    #[test]
    fn test_config_round_trip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.goal, parsed.goal);
        assert_eq!(config.max_iterations, parsed.max_iterations);
    }

    #[test]
    fn test_state_path() {
        let config = Config::default();
        assert_eq!(config.state_path(".openloop"), ".openloop/state.md");
    }
}