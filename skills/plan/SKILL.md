---
name: plan
description: Create or revise a speck through agent collaboration
disable-model-invocation: true
allowed-tools: Task
argument-hint: "\"idea\" or path/to/speck.md"
---

## Summary

Create or revise a speck through iterative agent collaboration. Invoke with `/specks:plan "your idea"` to create a new speck, or `/specks:plan path/to/speck.md` to revise an existing one.

## Your Role

You orchestrate the specks planning workflow by invoking the director agent with `mode=plan`. The director coordinates the interviewer, planner, and critic in an iterative loop until the user approves the speck.

## Invocation Modes

### New Speck from Idea

```
/specks:plan "add user authentication with OAuth2"
```

When given a quoted idea string, you enter **new speck mode**:
1. Director invokes clarifier skill to gather requirements
2. Director spawns interviewer agent to present questions to user
3. Director spawns planner agent to create the speck following the skeleton format
4. Director invokes critic skill to review for quality and completeness
5. Director spawns interviewer to present results and ask: "ready or revise?"
6. Loop continues until user approves

### Revision of Existing Speck

```
/specks:plan .specks/specks-auth.md
```

When given a path to an existing `.md` file, you enter **revision mode**:
1. Director spawns interviewer to present current speck state and ask what to change
2. Director spawns planner to revise the speck based on feedback
3. Director invokes critic skill to review the changes
4. Director spawns interviewer to present results and ask: "ready or revise?"
5. Loop continues until user approves

## How to Execute

Spawn the director agent with `mode=plan`:

```
Task(
  subagent_type: "specks:director",
  prompt: "mode=plan speck=\"$ARGUMENTS\"",
  description: "Plan speck"
)
```

Where `$ARGUMENTS` is either:
- A quoted idea string (e.g., `"add user authentication"`)
- A path to an existing speck (e.g., `.specks/specks-auth.md`)

## Detecting Input Mode

Determine the mode based on the input:

1. **Revision mode**: Input ends in `.md` AND the file exists
2. **New mode**: All other cases (idea string, non-existent file)

For revision mode, verify the file exists before invoking:
```
if input ends with ".md":
    check if file exists at:
    - input as absolute path
    - input relative to project root
    - input in .specks/ directory
    if file exists → revision mode
else → new mode
```

## Interactive Loop Behavior

The planning loop uses the interviewer agent for interactive dialogue:

1. **Gathering requirements**: Interviewer presents clarifier questions
2. **Presenting results**: Interviewer shows the speck summary and critic feedback
3. **Approval decision**: User can say:
   - "ready", "approve", "looks good", "yes" → approve and exit
   - "abort", "cancel", "quit" → abort the loop
   - Any other feedback → pass to planner for revision

**Important**: The loop runs until the user explicitly approves. There is no arbitrary iteration limit.

## Run Directory

A run directory is created at `.specks/runs/{session-id}/` containing:
- `metadata.json` - Session info and status
- `planning/` - Planning phase artifacts
  - `001-clarifier.json` - Clarifying questions generated
  - `002-interviewer.json` - User answers received
  - `003-planner.json` - Draft speck produced
  - `004-critic.json` - Quality review
  - etc.

## Exit Conditions

The planning loop exits when:
1. **Approved**: User approves the speck → status set to "active"
2. **Aborted**: User explicitly aborts → draft saved, exit cleanly

## Error Handling

| Error | Action |
|-------|--------|
| Speck file not found (revision mode) | Report error, suggest checking path |
| Validation errors in created speck | Automatically retry with planner |
| User abort | Save draft, exit cleanly |

## Example Usage

**Create a new speck:**
```
/specks:plan "add a REST API for user management"
```

**Revise an existing speck:**
```
/specks:plan .specks/specks-1.md
```

**Create with a detailed idea:**
```
/specks:plan "implement caching layer for API responses using Redis, with TTL support and cache invalidation"
```
