---
name: auditor
description: Check code quality, performance, and security
allowed-tools: Read, Grep, Glob
---

## Purpose

Check code quality, performance, and security of recent changes. This skill runs after implementation to catch issues before commit.

## Input

The director invokes this skill with a JSON payload:

```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "files_to_audit": ["string"],
  "drift_assessment": {
    "expected_files": ["string"],
    "actual_changes": ["string"],
    "unexpected_changes": [...],
    "drift_severity": "none|minor|moderate|major"
  }
}
```

**Fields:**
- `speck_path`: Path to the speck (for context)
- `step_anchor`: Which step was just completed
- `files_to_audit`: Files that were created or modified by implementer
- `drift_assessment`: From implementer output (for context on any unexpected changes)

## Output

Return JSON-only output (no prose, no markdown, no code fences):

```json
{
  "categories": {
    "structure": "PASS|WARN|FAIL",
    "error_handling": "PASS|WARN|FAIL",
    "security": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "severity": "critical|major|minor",
      "file": "string",
      "description": "string"
    }
  ],
  "drift_notes": "string | null",
  "recommendation": "APPROVE|FIX_REQUIRED|MAJOR_REVISION"
}
```

**Fields:**
- `categories.structure`: Code organization, naming, patterns
- `categories.error_handling`: Error handling completeness
- `categories.security`: Security considerations
- `issues`: List of problems found, ranked by severity
- `drift_notes`: Concerns about unexpected changes if drift occurred
- `recommendation`: Overall verdict

## Behavior

1. **Read the files**: Load each file in files_to_audit
2. **Check structure**: Code organization, naming conventions, patterns
3. **Check error handling**: Are errors handled? Propagated correctly?
4. **Check security**: Any obvious security issues?
5. **Review drift**: If unexpected changes occurred, are they concerning?
6. **Generate verdict**: APPROVE, FIX_REQUIRED, or MAJOR_REVISION

## Evaluation Criteria

### Structure
- **PASS**: Clean code, follows project conventions, good organization
- **WARN**: Minor style issues or slight pattern deviations
- **FAIL**: Significantly violates project conventions

### Error Handling
- **PASS**: Errors handled appropriately, good error messages
- **WARN**: Some error paths unclear or missing
- **FAIL**: Errors ignored or swallowed

### Security
- **PASS**: No obvious security issues
- **WARN**: Potential issues that should be reviewed
- **FAIL**: Clear security vulnerabilities

## Recommendation Logic

- **APPROVE**: All categories PASS, or minor WARNs with no critical/major issues
- **FIX_REQUIRED**: Major issues that implementer can fix quickly
- **MAJOR_REVISION**: Critical issues or fundamental design problems

## Drift Notes

If the implementer had minor drift, include `drift_notes` with any concerns about the unexpected changes. This flags potential creep for human awareness.

Example:
```json
"drift_notes": "Unexpected change to validation.rs appears safe - added helper function consistent with step goals. No security concerns."
```

## Example Output

```json
{
  "categories": {
    "structure": "PASS",
    "error_handling": "PASS",
    "security": "PASS"
  },
  "issues": [],
  "drift_notes": null,
  "recommendation": "APPROVE"
}
```

```json
{
  "categories": {
    "structure": "PASS",
    "error_handling": "WARN",
    "security": "PASS"
  },
  "issues": [
    {
      "severity": "major",
      "file": "crates/specks/src/commands/beads/sync.rs",
      "description": "Error from bd CLI is silently ignored on line 45"
    },
    {
      "severity": "minor",
      "file": "crates/specks/src/commands/beads/sync.rs",
      "description": "Consider using thiserror for custom error type"
    }
  ],
  "drift_notes": null,
  "recommendation": "FIX_REQUIRED"
}
```

```json
{
  "categories": {
    "structure": "WARN",
    "error_handling": "FAIL",
    "security": "WARN"
  },
  "issues": [
    {
      "severity": "critical",
      "file": "crates/specks/src/commands/execute.rs",
      "description": "User input passed directly to shell command without sanitization"
    },
    {
      "severity": "major",
      "file": "crates/specks/src/commands/execute.rs",
      "description": "All errors return generic 'operation failed' message"
    }
  ],
  "drift_notes": "Unexpected changes to shell execution logic raise security concerns",
  "recommendation": "MAJOR_REVISION"
}
```
