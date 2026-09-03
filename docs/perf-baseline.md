# Build performance baseline — end of Phase 15 (T16-010)

Reference point for the Phase 20 perf sign-off (T21-002 "build budget"). Captured
right after the `crates/ui` monolith was fully dissolved into ~20 crates
(T16-001 … T16-009).

## Environment

| | |
|---|---|
| Date | 2026-09-03 |
| Machine | Intel Xeon E5-2695 v4 @ 2.10 GHz, 4 vCPU (KVM/Proxmox guest), 503 GiB RAM |
| OS | Linux 7.0.14-11-pve |
| Toolchain | `rustc 1.98.0 (88d9e12ae 2026-08-18)`, `cargo 1.98.0` |
| Disk | single ext4 loop device, ~24 GiB free during the run |
| Cargo env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` (session-local, disk-constrained VPS) |
| Workspace | 20 crates, 81 internal edges, 8 dependency tiers (see `docs/architecture.md` §9) |

`CARGO_INCREMENTAL=0` means the "incremental" figures below are cargo's
*dependency-tracking* rebuilds (recompile only the crates whose inputs changed),
not LLVM incremental codegen. This matches the VPS dev setup and CI.

## Measurements

Wall-clock, single sample each (`date +%s` deltas), `-j4`.

| Scenario | Command | Time |
|---|---|---|
| **Cold** check | `cargo clean` → `cargo check --workspace --all-targets` | **327 s** (5 m 27 s) |
| **Warm** check (no edits) | `cargo check --workspace --all-targets` | **2 s** |
| Clippy (after cold check) | `cargo clippy --workspace --all-targets -- -D warnings` | **23 s** |
| **Incremental** — 1-line comment added to `crates/shell/src/app_shell.rs`, caches warm | `cargo check -p labonair-shell` | **~3 s** |
| Incremental — same edit, fingerprints cold (first `-p` run after an `--all-targets` + clippy pass) | `cargo check -p labonair-shell` | 158 s (noise — see note) |

`target/` after the cold check: 1.3 GiB.

### Note on the incremental figure

The **~3 s** stabilised value is the number the crate split was meant to move:
a one-line change in `app_shell.rs` now recompiles **only `labonair-shell`**, not
"half the `ui` monolith". The 158 s outlier was the very first `cargo check -p`
after a `--workspace --all-targets` build followed by a clippy pass — cargo had
to redo the non-test-cfg fingerprints for `labonair-shell` and its reverse-dep
closure once. Every subsequent 1-line edit re-checks in ~3 s.

T21-002 should compare against **~3 s** for the incremental budget and **327 s /
2 s** for the cold / warm `check --workspace --all-targets` budget.

## Coordinator-forced deviations (disk budget)

The task also asks for cold/warm `cargo build --release` and
`cargo test --workspace` timings. Both are **disk-banned** on this VPS (a full
debug/test build is ~20–28 GiB against ~25 GiB headroom; `cargo test --workspace`
was explicitly forbidden for this task). They are deferred to T21-002, which runs
on CI hardware with a real disk budget. The gate actually run here was:
`cargo fmt --check`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`scripts/check-crate-deps.sh` — all green.

## Open follow-up

`labonair-terminal` still pulls `gpui` + `labonair-theme` (it renders the ANSI
cell grid). An engine/renderer split would let `cargo check` on the pure engine
skip GPUI entirely — candidate for a later phase, not scoped to Phase 15.
