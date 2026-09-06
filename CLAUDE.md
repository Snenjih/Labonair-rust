# Labonair-rust — Claude Code Instructions

Use [`AGENTS.md`](AGENTS.md) as the engineering instruction set and [`docs/README.md`](docs/README.md) as the documentation index. The current product and architecture contracts are normative; archived reports and historical tasks are not active instructions.

Before changing a feature, identify its owning module, public contract, registries, settings, persistence, notifications, and UI-kit components. Keep feature behavior out of the shell and keep cross-module communication typed.

Every product capability starts in one canonical capability crate. Split it
into sibling crates only for a real dependency or lifecycle boundary; never
create a mixed feature crate or a second owner. New contributions are
registered by the owning module and wired by the composition root. Do not add
static shell-wide command, panel, theme, host, or notification tables.

Use the local `reference-src/` and `zed-refrence/` trees as read-only references. Never copy Zed implementation code into the project.
