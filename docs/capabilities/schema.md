# Capability Descriptor Schema

**Status:** Normative
**Version:** 1
**Related:** [`../capabilities.md`](../capabilities.md), [`README.md`](README.md), [`../agents/change-matrix.md`](../agents/change-matrix.md)

Capability descriptors under this directory are structured source records, not
free-form status claims. Each descriptor uses one `[capability]` table and
keeps current implementation status separate from target status.

## Required fields

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Stable machine identifier. |
| `display_name` | string | Human-readable capability name. |
| `owner_module` | string | Sole product owner. |
| `canonical_crate` | string | One canonical capability crate. |
| `sibling_crates` | array | Real contract/UI/storage/integration siblings under the same owner. |
| `public_contract` | array | Typed contracts, values, registries, or events consumed by others. |
| `canonical_entry_points` | array | Primary user-visible entry points. |
| `alternate_entry_points` | array | Supported secondary entry points, or `none`. |
| `commands` | array | Stable command IDs, or `none`. |
| `default_keymap` | array | Default bindings, or `none`. |
| `registries` | array | Registries contributed to or owned, or `none`. |
| `events` | array | Typed events/actions exchanged, or `none`. |
| `state_owner` | string | Runtime state owner. |
| `persistence_owner` | string | Persistence owner and storage, or `none`. |
| `settings` | array | Settings IDs/ownership, or `none`. |
| `notifications` | array | Notification behavior and owner, or `none`. |
| `ui_kit_components` | array | Shared UI-kit components, or `none`. |
| `automation_tools` | array | Supported automation/MCP tools, or `none`. |
| `security_boundaries` | array | Secret, path, grant, host, or trust constraints. |
| `source_files` | array | Repository paths that anchor the descriptor. |
| `tests` | array | Exact test targets or test-bearing source paths. |
| `visual_states` | array | Required visual states and evidence status. |
| `performance_budget` | string | Numeric budget or explicit not-applicable rationale. |
| `current_status` | string | What current evidence proves. |
| `target_status` | string | Intended completion state. |
| `known_limitations` | array | Open gaps; never hide missing evidence. |
| `removal_condition` | string | Compatibility removal condition, or `none`. |
| `next_task` | repository path | Bounded active-queue task that owns the next descriptor/evidence step. |
| `last_verified` | string | ISO date of the descriptor audit. |

The `[evidence]` table is required and contains arrays for `contract`, `unit`,
`integration`, `command`, `surface`, `persistence`, `notification`, `visual`,
`security`, `performance`, and `regression`. Evidence entries must identify a
test, source path, artifact, or explicit `Pending`/`N/A` reason.

Allowed status values are `planned`, `contract-defined`, `implemented`,
`integration-tested`, `visually-verified`, `security-reviewed`,
`performance-baselined`, `regression-locked`, `partial`, `deferred`, and
`removed`.
