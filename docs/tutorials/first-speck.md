# Tutorial: Create Your First Speck

This tutorial walks you through creating your first speck using the iterative planning loop.

## What You'll Learn

- How to start the planning workflow
- How the interviewer-planner-critic loop works
- How to provide feedback and iterate
- How to approve and finalize a speck

## Prerequisites

- specks installed and initialized (`specks init`)
- Claude Code installed and configured
- A project you want to plan changes for

## Scenario

Let's say you want to add a simple feature: a `/health` endpoint to your API that returns status information. We'll use the planning workflow to create a structured implementation plan.

## Step 1: Start the Planning Loop

Open your terminal in your project directory and run:

```bash
specks plan "add a health check endpoint that returns server status"
```

Alternatively, if you're inside a Claude Code session:

```
/specks-plan "add a health check endpoint that returns server status"
```

## Step 2: Answer the Interviewer's Questions

The interviewer agent will ask questions to understand your requirements. You might see questions like:

> **Interviewer**: I'm gathering requirements for your health check endpoint. Let me ask a few questions:
>
> 1. What framework is your API built with (Express, FastAPI, etc.)?
> 2. What information should the health check return (just "ok" status, or also database connectivity, memory usage, etc.)?
> 3. Should the endpoint require authentication?
> 4. Is there a specific path you prefer (e.g., `/health`, `/api/health`, `/_health`)?

Answer each question thoughtfully. The more context you provide, the better the resulting speck will be.

**Example response:**

> The API uses Express.js. The health check should return a JSON object with:
> - `status`: "ok" or "error"
> - `timestamp`: current ISO timestamp
> - `database`: connection status (connected/disconnected)
> - `uptime`: server uptime in seconds
>
> The endpoint should be at `/health` and not require authentication.

## Step 3: Review the Generated Speck

After gathering requirements, the planner creates a speck. The critic then reviews it for:

- Completeness (all required sections present)
- Implementability (steps are actionable)
- Proper sequencing (dependencies make sense)
- Clear scope (not too ambitious or trivial)

The interviewer presents the results with a **punch list** of any concerns:

> **Interviewer**: The planner has created a speck for your health check endpoint. Here's a summary:
>
> **Phase: Add Health Check Endpoint**
> - 3 execution steps planned
> - Step 0: Create health service module
> - Step 1: Add health check route
> - Step 2: Add integration tests
>
> **Critic's assessment**: APPROVE with minor suggestions
>
> **Punch list (open items)**:
> - [ ] Consider adding a timeout for database connectivity check
> - [ ] Specify expected response time SLA
>
> What would you like to do?
> - Say "ready" or "approve" to finalize
> - Provide feedback to revise specific aspects
> - Say "abort" to cancel

## Step 4: Iterate on Feedback

If you have concerns, provide feedback:

> Please add a 5-second timeout for the database connectivity check, and add a note that the endpoint should respond within 200ms under normal conditions.

The planner revises the speck with your feedback, and the critic reviews again. The loop continues until you're satisfied.

## Step 5: Approve the Speck

When you're happy with the speck, say "ready" or "approve":

> ready

The interviewer confirms:

> **Interviewer**: Great! The speck has been finalized and saved to `.specks/specks-health-check.md` with status "active".
>
> You can:
> - View the full speck: `cat .specks/specks-health-check.md`
> - Validate the speck: `specks validate specks-health-check.md`
> - Execute the speck: `specks execute .specks/specks-health-check.md`

## Understanding the Result

Open the generated speck to see its structure:

```bash
cat .specks/specks-health-check.md
```

You'll see sections like:

```markdown
## Phase X.Y: Add Health Check Endpoint {#phase-health-check}

**Purpose:** Add a `/health` endpoint that returns server status information...

### Plan Metadata {#plan-metadata}

| Field | Value |
|-------|-------|
| Owner | your-name |
| Status | active |
| Target branch | main |
...

### Phase Overview {#phase-overview}

#### Context {#context}

The API currently has no health monitoring endpoint...

#### Success Criteria (Measurable) {#success-criteria}

- GET /health returns 200 OK with JSON body...
- Response time under 200ms for healthy state...

### 2.0.0 Design Decisions {#design-decisions}

#### [D01] Use separate health service module (DECIDED) {#d01-health-service}

**Decision:** Create a dedicated health service module...

### 2.0.5 Execution Steps {#execution-steps}

#### Step 0: Create Health Service Module {#step-0}
...

#### Step 1: Add Health Check Route {#step-1}

**Depends on:** #step-0
...
```

## Tips for Effective Planning

### Be Specific About Requirements

Instead of: "add health check"

Say: "add a health check endpoint at /health that returns JSON with status, database connectivity, and uptime"

### Mention Constraints

If you have specific requirements, mention them upfront:
- "The endpoint must respond within 200ms"
- "Use the existing database connection pool"
- "Follow our REST API naming conventions"

### Review the Punch List

The interviewer's punch list highlights open concerns. Don't ignore items—either address them or explicitly decide they're not needed.

### Iterate Until Satisfied

There's no penalty for multiple iterations. Keep providing feedback until the speck accurately captures what you want to build.

## Revising an Existing Speck

You can re-enter the planning loop for any speck:

```bash
specks plan .specks/specks-health-check.md
```

This opens the speck in **revision mode**. The interviewer presents the current state and asks what you want to change.

## Next Steps

Now that you have an active speck:

1. **Validate it**: `specks validate specks-health-check.md`
2. **Execute it**: See [Execute a Plan](execute-plan.md) tutorial
3. **Track progress**: `specks status specks-health-check.md`

## Common Issues

### Speck Validation Fails

If the generated speck has validation issues, the planner automatically retries. If problems persist, try being more specific in your requirements.

### Loop Seems Stuck

Say "abort" to exit cleanly. The draft is saved and you can review what was generated.

### Too Much Iteration

If you're going back and forth too many times, consider:
- Providing all requirements upfront
- Writing a brief spec document and using `--context spec.md`
- Starting fresh with clearer requirements
