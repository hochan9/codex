# HANDOFF

## Context
- Date: 2026-02-11
- Goal: production-grade sub-agent workflow for Codex (Claude-style sub-agent UX + deterministic orchestration in core).
- This session finalized:
  - core-level deterministic orchestration controls
  - fixed real-time sub-agent board panel in TUI
  - `sub-codex` / `codex-agent` alias invocation behavior in CLI
  - stabilization fixes discovered during regression tests

## What Was Completed

### 1) Core orchestration promoted from prompt hint to runtime policy
- Files:
  - `codex-rs/core/src/state/turn.rs`
  - `codex-rs/core/src/state/mod.rs`
  - `codex-rs/core/src/tools/handlers/collab.rs`
  - `codex-rs/core/src/tools/spec.rs`
- Added turn-scoped deterministic orchestration state:
  - `CollabDelegationPolicy` / `CollabDelegationIntent`
  - tracked known workers, pending spawns, spawn attempts, wait timeout rounds
- Added runtime controls:
  - worker budget accounting and hard-limit enforcement (`agent_max_threads` bound)
  - spawn reservation + retry loop for retryable failures
  - timeout escalation policy in `wait` when fallback omitted (auto close stalled workers after repeated timeout rounds)
  - cleanup hooks when workers close/fail
- Updated collab tool descriptions to reflect core enforcement behavior.

### 2) TUI board moved to fixed panel (not history cell dedupe)
- Files:
  - `codex-rs/tui/src/chatwidget.rs`
  - `codex-rs/tui/src/collab.rs`
  - `codex-rs/tui/src/chatwidget/tests.rs`
- `Sub-agent board` now renders as a persistent panel between active content and bottom pane.
- History spam path for board snapshots was removed.
- Delegation policy prompt injection into developer instructions was removed; policy now lives in core runtime logic.
- Added/updated tests for fixed board rendering and policy-injection removal.

### 3) Alias-aware CLI invocation behavior (`sub-codex`, `codex-agent`)
- File:
  - `codex-rs/cli/src/main.rs`
- Added alias detection from `argv[0]` and alias-aware help/usage/completion naming.
- Added alias defaults:
  - invoking via `sub-codex` or `codex-agent` prepends low-precedence `features.collab=true` override.
- Added startup banner for alias interactive runs:
  - `Launching <alias> with sub-agent defaults (features.collab=true).`
- Added tests for alias usage/help and default override ordering.

### 4) Stability fixes found during regression
- Files:
  - `codex-rs/tui/src/render/renderable.rs`
  - `codex-rs/tui/tests/suite/no_panic_on_startup.rs`
  - `codex-rs/core/src/state/turn.rs` (test expectation correction)
- Fixed TUI overflow panic in `InsetRenderable::desired_height` by using saturating math.
- Added regression test: `inset_desired_height_saturates_narrow_width`.
- Fixed `no_panic_on_startup` integration timeout caused by migration prompt interception by pre-seeding:
  - `[notice.model_migrations] "gpt-5.2-codex" = "gpt-5.3-codex"`
- Corrected worker-budget unit test to match actual budget semantics.

## Verification Run (Latest)
- `just fmt`
  - passed.
- `cargo test -p codex-core`
  - passed (elevated run used to satisfy wiremock/port constraints in this environment).
- `cargo test -p codex-tui`
  - passed (including previously failing `suite::no_panic_on_startup::malformed_rules_should_not_panic`).
- `cargo test -p codex-cli`
  - passed.
- `just fix -p codex-core`
  - passed (elevated run required in this environment).
- `just fix -p codex-tui`
  - passed (elevated run required in this environment).
- `just fix -p codex-cli`
  - passed (elevated run required in this environment).

## Important Note
- Per repo rule, full workspace regression was **not** run yet:
  - `cargo test --all-features`
- Because `core` changed, this should be run only after product-owner approval.

## Current Known State
- Feature behavior is now deterministic in core for spawn budgeting/retry/wait escalation.
- TUI board is fixed/persistent and reflects active/queued/done worker summary in real time.
- Alias invocation has user-facing identity (`sub-codex` / `codex-agent`) in usage/help/banner/completions.

## Next Session Quick Start
1. `cd /Users/sonhochan/IdeaProjects/codex/codex-rs`
2. Smoke test:
   - run `codex` (baseline behavior)
   - run `sub-codex` (collab default + alias banner/help identity)
   - run a multi-step/multi-domain prompt and observe fixed `Sub-agent board` updates.
3. If approved, run workspace regression:
   - `cargo test --all-features`

## Recommended Next Steps (V2)
1. Extend deterministic policy from per-call controls to full task scheduler (priority queue + explicit worker assignment strategy).
2. Add end-to-end tests for automatic split/retry/timeout paths at collab workflow level.
3. Add explicit user controls for orchestration strategy (`aggressive`, `balanced`, `conservative`) with predictable budget profiles.
4. Finalize packaging/docs so `sub-codex` is documented as first-class entrypoint in install/release notes.
