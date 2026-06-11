use std::collections::HashMap;

use anyhow::Result;

use super::types::AgentConfig;

#[allow(dead_code)]
const KNOWN_AGENTS: &[&str] = &["opencode", "copilot", "claude", "aider", "sweep"];

#[allow(dead_code)]
pub fn discover_agents() -> Result<Vec<AgentConfig>> {
    let mut agents = Vec::new();

    for name in KNOWN_AGENTS {
        if is_installed(name) {
            agents.push(AgentConfig {
                name: name.to_string(),
                model: default_model(name),
                model_config: HashMap::new(),
            });
        }
    }

    Ok(agents)
}

#[allow(dead_code)]
pub fn is_installed(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[allow(dead_code)]
pub fn default_model(agent: &str) -> String {
    match agent {
        "opencode" => "claude-sonnet-4-20250514",
        "copilot" => "gpt-4o",
        "claude" => "claude-sonnet-4-20250514",
        "aider" => "claude-sonnet-4-20250514",
        "sweep" => "gpt-4o",
        _ => "default",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_agents_nonempty() {
        assert!(!KNOWN_AGENTS.is_empty());
    }

    #[test]
    fn test_default_model_for_known() {
        let model = default_model("opencode");
        assert!(!model.is_empty());
    }

    #[test]
    fn test_default_model_for_unknown() {
        let model = default_model("unknown-cli");
        assert_eq!(model, "default");
    }
}