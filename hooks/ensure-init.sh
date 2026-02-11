#!/bin/bash
# Ensure specks is initialized before specks agents run.
# Hooks PreToolUse:Task — fires right before the first agent is spawned.
# Runs `specks init --quiet` as a side effect — milliseconds, no API call.
# Idempotent: if .specks/ exists, specks init is a no-op.

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name')

if [ "$TOOL_NAME" = "Task" ]; then
  AGENT_TYPE=$(echo "$INPUT" | jq -r '.tool_input.subagent_type // empty')
  if [[ "$AGENT_TYPE" == specks:* ]]; then
    specks init --quiet 2>/dev/null || true
  fi
fi

# No JSON output — don't interfere with permission decisions
exit 0
