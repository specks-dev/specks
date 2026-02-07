---
name: critic
description: Review speck quality and implementability
allowed-tools: Read, Grep, Glob
---

## Purpose

Review a speck for skeleton compliance, quality, and implementability. This skill ensures specks meet quality standards before execution begins.

## Input

The director invokes this skill with a JSON payload:

```json
{
  "speck_path": "string",
  "skeleton_path": "string"
}
```

**Fields:**
- `speck_path`: Path to the speck to review (required)
- `skeleton_path`: Path to skeleton template for compliance check (defaults to `.specks/specks-skeleton.md`)

## Output

Return JSON-only output (no prose, no markdown, no code fences):

```json
{
  "skeleton_compliant": true,
  "areas": {
    "completeness": "PASS|WARN|FAIL",
    "implementability": "PASS|WARN|FAIL",
    "sequencing": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "priority": "HIGH|MEDIUM|LOW",
      "description": "string"
    }
  ],
  "recommendation": "APPROVE|REVISE|REJECT"
}
```

**Fields:**
- `skeleton_compliant`: Whether the speck follows the skeleton format
- `areas.completeness`: Are all required sections present and filled in?
- `areas.implementability`: Can each step actually be implemented?
- `areas.sequencing`: Are dependencies correct? Is the order logical?
- `issues`: List of problems found, prioritized
- `recommendation`: Overall verdict

## Behavior

1. **Read the skeleton**: Understand the required structure
2. **Read the speck**: Load the speck to review
3. **Check structure compliance**: Compare against skeleton
4. **Evaluate completeness**: Are all sections filled in adequately?
5. **Assess implementability**: Can each step be executed?
6. **Verify sequencing**: Are dependencies declared and logical?
7. **Generate verdict**: APPROVE, REVISE, or REJECT

## Evaluation Criteria

### Completeness
- **PASS**: All required sections present with meaningful content
- **WARN**: Minor gaps or thin sections
- **FAIL**: Missing required sections or empty content

### Implementability
- **PASS**: Each step has clear tasks, artifacts, and checkpoints
- **WARN**: Some steps vague but recoverable
- **FAIL**: Steps are too abstract to implement

### Sequencing
- **PASS**: Dependencies are correct, order is logical
- **WARN**: Minor ordering issues
- **FAIL**: Circular dependencies or broken references

## Recommendation Logic

- **APPROVE**: All areas PASS, or minor WARNs with no HIGH priority issues
- **REVISE**: Any area has WARN with HIGH priority issues, or any FAIL that's fixable
- **REJECT**: Multiple FAILs or fundamental structural problems

## Example Output

```json
{
  "skeleton_compliant": true,
  "areas": {
    "completeness": "PASS",
    "implementability": "WARN",
    "sequencing": "PASS"
  },
  "issues": [
    {
      "priority": "MEDIUM",
      "description": "Step 3 lacks specific test criteria"
    },
    {
      "priority": "LOW",
      "description": "Non-goals section could be more explicit"
    }
  ],
  "recommendation": "APPROVE"
}
```

```json
{
  "skeleton_compliant": false,
  "areas": {
    "completeness": "FAIL",
    "implementability": "WARN",
    "sequencing": "PASS"
  },
  "issues": [
    {
      "priority": "HIGH",
      "description": "Missing Plan Metadata section"
    },
    {
      "priority": "HIGH",
      "description": "No execution steps defined"
    },
    {
      "priority": "MEDIUM",
      "description": "Success criteria are vague"
    }
  ],
  "recommendation": "REJECT"
}
```
