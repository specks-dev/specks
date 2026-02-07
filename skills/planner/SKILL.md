---
name: planner
description: Entry point for planning workflow - spawns planner-agent
allowed-tools: Task
---

## Purpose

Thin entry point that immediately spawns the planner-agent.

## Usage

/specks:planner "add user authentication"
/specks:planner .specks/specks-auth.md
/specks:planner --resume 20260206-143022-plan-a1b2c3
/specks:planner {"idea":"add user authentication","session_id":null}

## Behavior

Immediately spawn the planner-agent:

Task(subagent_type: "specks:planner-agent", prompt: "$ARGUMENTS", description: "Run planning loop")

Do NOT do any setup, validation, or processing. The planner-agent handles everything.

*Note:* This call is synchronous: the entry skill returns when the planner-agent finishes, preserving the "no fire-and-forget" principle.
