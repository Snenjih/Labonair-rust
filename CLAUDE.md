# Labonair-rust — Claude Code Instructions

Use [`AGENTS.md`](AGENTS.md) as the engineering instruction set and [`docs/README.md`](docs/README.md) as the documentation index. The current product and architecture contracts are normative; archived reports and historical tasks are not active instructions.

Before changing a feature, identify its owning module, public contract, registries, settings, persistence, notifications, and UI-kit components. Keep feature behavior out of the shell and keep cross-module communication typed.

Use the local `reference-src/` and `zed-refrence/` trees as read-only references. Never copy Zed implementation code into the project.
