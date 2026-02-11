## Summary

- **Step 0** (c27fd296): Added has_remote_origin() helper and extended MergeData with merge_mode, squash_commit, would_squash_merge fields
- **Step 1** (b47620b0): Added squash_merge_branch() helper with conflict detection, git reset --merge recovery, and integration tests
- **Step 2** (fcd3678b): Wired local merge mode into run_merge() — mode detection, conditional branching, empty merge pre-check, 4 integration tests
- **Step 3** (b9c85ba1): Updated CLI help text and merge skill documentation for dual-mode merge

## Test plan

All tests passed during implementation:
- Step 0: 322 tests passed
- Step 1: 326 tests passed
- Step 2: 330 tests passed (includes 4 new local merge integration tests)
- Step 3: 330 tests passed

🤖 Generated with [Claude Code](https://claude.com/claude-code)
