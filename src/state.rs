use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub iteration: u32,
    pub goal_path: String,
    pub last_plan: String,
    pub last_result: String,
    pub goal_met: bool,
    pub timestamp: String,
}

#[allow(dead_code)]
impl LoopState {
    pub fn new(goal_path: &str) -> Self {
        LoopState {
            iteration: 0,
            goal_path: goal_path.to_string(),
            last_plan: String::new(),
            last_result: String::new(),
            goal_met: false,
            timestamp: chrono_now(),
        }
    }

    pub fn load(state_path: &str) -> Result<Self> {
        let path = Path::new(state_path);
        if !path.exists() {
            return Ok(LoopState::new(""));
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", state_path))?;

        // Try JSON first, fall back to markdown parsing
        if let Ok(state) = serde_json::from_str::<LoopState>(&contents) {
            return Ok(state);
        }

        // Fallback: parse state.md format
        Ok(parse_state_md(&contents, state_path))
    }

    pub fn save(&self, state_path: &str) -> Result<()> {
        let path = Path::new(state_path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }

        // Write as markdown for human readability
        let md = format!(
            "# Loop State\n\n\
             - Iteration: {}\n\
             - Goal: {}\n\
             - Goal Met: {}\n\
             - Last Updated: {}\n\
             \n\
             ## Last Plan\n\
             {}\n\
             \n\
             ## Last Result\n\
             {}\n",
            self.iteration,
            self.goal_path,
            self.goal_met,
            self.timestamp,
            self.last_plan,
            self.last_result,
        );

        std::fs::write(path, &md)
            .with_context(|| format!("Failed to write {}", state_path))?;

        Ok(())
    }

    pub fn increment(&mut self) {
        self.iteration += 1;
        self.timestamp = chrono_now();
    }
}

fn parse_state_md(contents: &str, _path: &str) -> LoopState {
    let mut state = LoopState::new("");

    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("- Iteration: ") {
            state.iteration = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("- Goal: ") {
            state.goal_path = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("- Goal Met: ") {
            state.goal_met = val.trim().parse().unwrap_or(false);
        } else if let Some(val) = line.strip_prefix("- Last Updated: ") {
            state.timestamp = val.trim().to_string();
        }
    }

    state
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Simple UTC timestamp without chrono dependency
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", 1970 + days / 365, 1, 1, hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = LoopState::new("GOAL.md");
        assert_eq!(state.iteration, 0);
        assert!(!state.goal_met);
        assert_eq!(state.goal_path, "GOAL.md");
    }

    #[test]
    fn test_increment() {
        let mut state = LoopState::new("GOAL.md");
        state.increment();
        assert_eq!(state.iteration, 1);
        state.increment();
        assert_eq!(state.iteration, 2);
    }

    #[test]
    fn test_save_and_load() {
        let tmp = "/tmp/test-openloop-state.md";
        let mut state = LoopState::new("GOAL.md");
        state.iteration = 5;
        state.goal_met = true;
        state.last_plan = "Do step 1".into();
        state.save(tmp).unwrap();

        let loaded = LoopState::load(tmp).unwrap();
        assert_eq!(loaded.iteration, 5);
        assert!(loaded.goal_met);
        assert_eq!(loaded.goal_path, "GOAL.md");
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_load_nonexistent() {
        let state = LoopState::load("/tmp/nonexistent-state-file.md").unwrap();
        assert_eq!(state.iteration, 0);
    }

    #[test]
    fn test_parse_state_md() {
        let contents = "\
# Loop State

- Iteration: 3
- Goal: GOAL.md
- Goal Met: true
- Last Updated: 2026-01-01T00:00:00Z
";
        let state = parse_state_md(contents, "test.md");
        assert_eq!(state.iteration, 3);
        assert!(state.goal_met);
        assert_eq!(state.goal_path, "GOAL.md");
    }
}