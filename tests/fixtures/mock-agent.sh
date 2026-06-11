#!/usr/bin/env bash
# Mock agent for integration tests
# Responds with canned output based on the prompt content

set -e

MODEL=""
PROMPT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --model) MODEL="$2"; shift 2 ;;
        *) PROMPT="$*"; break ;;
    esac
done

if [[ -z "$PROMPT" ]]; then
    read -r PROMPT
fi

if echo "$PROMPT" | grep -qi "clarifying questions"; then
    cat << 'EOF'
What platform is the target?
What is the expected timeline?
EOF
elif echo "$PROMPT" | grep -qi "GOAL.md"; then
    cat << 'EOF'
# Project Goal

Build a CLI tool.

## Success Criteria

- [ ] Criterion 1 met
EOF
elif echo "$PROMPT" | grep -qi "single most impactful"; then
    cat << 'EOF'
SUMMARY: Implement the core feature
TASKS:
- Add the feature
- Test it
EOF
elif echo "$PROMPT" | grep -qi "verify"; then
    cat << 'EOF'
GOAL_MET: false
REASON: Feature not yet complete
REMAINING:
- Finish implementation
EOF
else
    echo "Mock agent executed with model: $MODEL"
    echo "Prompt: ${PROMPT:0:100}..."
fi