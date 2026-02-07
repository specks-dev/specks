#!/bin/bash
# Auto-approve specks plugin Skill and Task invocations
#
# For Skill tool: checks if skill name starts with "specks:"
# For Task tool: checks if subagent_type starts with "specks:"

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name')

# Check if this is a specks component
IS_SPECKS=false

if [ "$TOOL_NAME" = "Skill" ]; then
  SKILL_NAME=$(echo "$INPUT" | jq -r '.tool_input.skill // empty')
  if [[ "$SKILL_NAME" == specks:* ]]; then
    IS_SPECKS=true
  fi
elif [ "$TOOL_NAME" = "Task" ]; then
  AGENT_TYPE=$(echo "$INPUT" | jq -r '.tool_input.subagent_type // empty')
  if [[ "$AGENT_TYPE" == specks:* ]]; then
    IS_SPECKS=true
  fi
fi

if [ "$IS_SPECKS" = true ]; then
  jq -n '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "allow",
      permissionDecisionReason: "Specks plugin component auto-approved"
    }
  }'
else
  # Not a specks component - let normal permission flow handle it
  exit 0
fi
