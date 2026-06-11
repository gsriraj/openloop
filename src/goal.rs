use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Goal {
    pub raw: String,
    pub path: String,
}

#[allow(dead_code)]
impl Goal {
    pub fn from_file(path: &str) -> Result<Self> {
        let goal_path = Path::new(path);
        if !goal_path.exists() {
            anyhow::bail!("Goal file not found: {}", path);
        }

        let raw = std::fs::read_to_string(goal_path)
            .with_context(|| format!("Failed to read {}", path))?;

        Ok(Goal {
            raw,
            path: path.to_string(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.raw.trim().is_empty()
    }

    pub fn summary(&self) -> String {
        self.raw
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").to_string())
            .unwrap_or_else(|| "Untitled goal".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_goal_summary() {
        let goal = Goal {
            raw: "# My Project\n\nDo stuff".into(),
            path: "test.md".into(),
        };
        assert_eq!(goal.summary(), "My Project");
    }

    #[test]
    fn test_goal_summary_fallback() {
        let goal = Goal {
            raw: "Just some text".into(),
            path: "test.md".into(),
        };
        assert_eq!(goal.summary(), "Untitled goal");
    }

    #[test]
    fn test_goal_is_empty() {
        let goal = Goal {
            raw: "   ".into(),
            path: "test.md".into(),
        };
        assert!(goal.is_empty());
    }

    #[test]
    fn test_goal_not_empty() {
        let goal = Goal {
            raw: "content".into(),
            path: "test.md".into(),
        };
        assert!(!goal.is_empty());
    }

    #[test]
    fn test_from_file_not_found() {
        let result = Goal::from_file("/tmp/nonexistent-file-for-test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_success() {
        let tmp = "/tmp/test-openloop-goal.md";
        let mut f = std::fs::File::create(tmp).unwrap();
        writeln!(f, "# Test Goal").unwrap();
        let goal = Goal::from_file(tmp).unwrap();
        assert_eq!(goal.summary(), "Test Goal");
        let _ = std::fs::remove_file(tmp);
    }
}
