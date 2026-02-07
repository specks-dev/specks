---
name: implementer
description: Entry point for implementation workflow - spawns implementer-agent
allowed-tools: Task
---

## Purpose

Thin entry point that immediately spawns the implementer-agent.

## Usage

/specks:implementer .specks/specks-3.md
/specks:implementer .specks/specks-3.md --start-step #step-2
/specks:implementer .specks/specks-3.md --commit-policy manual
/specks:implementer .specks/specks-3.md --resume 20260206-150145-impl-d4e5f6
/specks:implementer {"speck_path":".specks/specks-3.md","commit_policy":"auto","session_id":null}

## Behavior

Immediately spawn the implementer-agent:

Task(subagent_type: "specks:implementer-agent", prompt: "$ARGUMENTS", description: "Run implementation loop")

Do NOT do any setup, validation, or processing. The implementer-agent handles everything.

*Note:* This call is synchronous: the entry skill returns when the implementer-agent finishes, preserving the "no fire-and-forget" principle.
