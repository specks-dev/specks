---
name: specks-plan
description: |
  Create or revise a speck through iterative agent collaboration.
  Use to transform an idea into a structured implementation plan, or to revise an existing speck.
argument-hint: "\"idea\" or path/to/speck.md"
---

## Summary

Create or revise a speck through iterative agent collaboration. Invoke with `/specks-plan "your idea"` to create a new speck, or `/specks-plan path/to/speck.md` to revise an existing one.

## Your Role

You orchestrate the specks planning workflow by invoking the director agent with `mode=plan`. The director coordinates the interviewer, planner, and critic agents in an iterative loop until the user approves the speck.

## Invocation Modes

### New Speck from Idea

```
/specks-plan "add user authentication with OAuth2"
```

When given a quoted idea string, you enter **new speck mode**:
1. Director invokes interviewer to gather requirements
2. Planner creates the speck following the skeleton format
3. Critic reviews for quality and completeness
4. Interviewer presents results and asks: "ready or revise?"
5. Loop continues until user approves

### Revision of Existing Speck

```
/specks-plan .specks/specks-auth.md
```

When given a path to an existing `.md` file, you enter **revision mode**:
1. Interviewer presents current speck state and asks what to change
2. Planner revises the speck based on feedback
3. Critic reviews the changes
4. Interviewer presents results and asks: "ready or revise?"
5. Loop continues until user approves

## Workflow

```
/specks-plan [idea OR existing-speck-path]
         |
    INTERVIEWER (gather initial input OR revision feedback)
         |
    PLANNER (create or revise speck)
         |
    CRITIC (review for quality and compliance)
         |
    INTERVIEWER (present result with punch list, ask: "ready or revise?")
         |
    user says ready? --> speck status = active, done
    user has feedback? --> loop back to planner with feedback
```

## How to Execute

Invoke the director agent with `mode=plan`:

```
Task(
  subagent_type: "specks-director",
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

The planning loop uses the AskUserQuestion tool for interactive dialogue:

1. **Gathering requirements**: Interviewer asks clarifying questions
2. **Presenting results**: Interviewer shows the speck summary and critic feedback
3. **Approval decision**: User can say:
   - "ready", "approve", "looks good", "yes" → approve and exit
   - "abort", "cancel", "quit" → abort the loop
   - Any other feedback → pass to planner for revision

**Important**: The loop runs until the user explicitly approves. There is no arbitrary iteration limit.

## Punch List

The interviewer maintains a punch list of open items:
- Items come from: critic feedback, interviewer analysis, user concerns
- Items are checked off when addressed
- Each iteration presents: "Here is what I think still needs attention: [list]"
- User can focus on specific items or approve if satisfied

## Run Directory

A run directory is created at `.specks/runs/{uuid}/` containing:
- `invocation.json` - Director parameters
- `critic-report.md` - Critic's assessment
- `architect-plan.md` - Implementation strategy (if planning continues to architecture)
- `status.json` - Final outcome

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
/specks-plan "add a REST API for user management"
```

**Revise an existing speck:**
```
/specks-plan .specks/specks-1.md
```

**Create with a detailed idea:**
```
/specks-plan "implement caching layer for API responses using Redis, with TTL support and cache invalidation"
```

## Integration with CLI

This skill provides the same functionality as `specks plan` from the command line:

| CLI Command | Equivalent Skill |
|-------------|------------------|
| `specks plan "idea"` | `/specks-plan "idea"` |
| `specks plan .specks/specks-1.md` | `/specks-plan .specks/specks-1.md` |

Both paths invoke the same director workflow and produce identical outcomes.
