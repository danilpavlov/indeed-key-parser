# SDD ledger — plan: docs/superpowers/plans/2026-08-31-indeed-key-code-parser.md

Mode: B (degraded) — Bash blocked by auto-mode classifier. Implementers Write-only;
no test runs, no git commits. User runs cargo test / gradlew and commits.

Ruling (preflight): SDD bash machinery (workspace/brief/review-package scripts, per-task
commits, test runs, worktree) unavailable. Adapted: subagents create files via Write only;
controller reviews file output; user verifies by building/testing. Cost if wrong: code may
not compile until user's first build; fixed from build output.

Preflight conflict scan:
- Tasks 1-4 (server, Rust) touch only server/**; Tasks 5-9 (android, Kotlin) touch only
  android/**. No shared files between the two trees. Interfaces within server chain
  (config -> db -> auth/validate -> app) consistent (CodeRecord, normalize_code,
  is_authorized signatures match across tasks 2/3/4). Android Entry/UiNode consistent
  across tasks 5/6/8/9. Scan clean.

Progress:
