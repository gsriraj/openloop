use std::collections::HashMap;

use anyhow::Result;

use super::types::AgentConfig;

const KNOWN_AGENTS: &[&str] = &["opencode", "copilot", "claude", "aider", "sweep"];

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

pub fn discover_models(agent_name: &str) -> Result<Vec<String>> {
    match agent_name {
        "opencode" => discover_opencode_models(),
        "copilot" => Ok(curated_copilot_models()),
        "claude" => Ok(curated_claude_models()),
        _ => Ok(vec![default_model(agent_name)]),
    }
}

fn discover_opencode_models() -> Result<Vec<String>> {
    let output = std::process::Command::new("opencode")
        .arg("models")
        .output()?;

    if !output.status.success() {
        return Ok(fallback_opencode_models());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.contains("No models"))
        .collect();

    if models.is_empty() {
        Ok(fallback_opencode_models())
    } else {
        Ok(models)
    }
}

fn fallback_opencode_models() -> Vec<String> {
    vec![
        "openrouter/anthropic/claude-sonnet-4".into(),
        "openrouter/anthropic/claude-haiku-4.5".into(),
        "openrouter/anthropic/claude-opus-4".into(),
        "openrouter/openai/gpt-4o".into(),
        "openrouter/openai/gpt-4o-mini".into(),
        "openrouter/google/gemini-flash-latest".into(),
        "openrouter/deepseek/deepseek-v4-flash".into(),
    ]
}

fn curated_copilot_models() -> Vec<String> {
    vec![
        "gpt-4o".into(),
        "gpt-4o-mini".into(),
        "gpt-5.2".into(),
        "claude-sonnet-4-20250514".into(),
        "claude-3.5-sonnet".into(),
    ]
}

fn curated_claude_models() -> Vec<String> {
    vec![
        "claude-sonnet-4-20250514".into(),
        "claude-opus-4-20250514".into(),
        "claude-haiku-4-20250514".into(),
    ]
}

pub fn is_installed(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn default_model(agent: &str) -> String {
    match agent {
        "opencode" => "openrouter/anthropic/claude-sonnet-4",
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

    #[test]
    fn test_fallback_opencode_models() {
        let models = fallback_opencode_models();
        assert!(!models.is_empty());
        assert!(models[0].contains("claude"));
    }

    #[test]
    fn test_curated_copilot_models() {
        let models = curated_copilot_models();
        assert!(!models.is_empty());
    }
}
