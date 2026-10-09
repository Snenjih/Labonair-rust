# Agent Anti-Patterns

These are repository-level failure modes. Remove the cause rather than adding
another compatibility layer around it.

## Ownership and architecture

- placing feature state or feature behavior in `labonair-shell`, `labonair`, or
  generic Workspace orchestration;
- inferring ownership from the caller instead of `docs/capabilities.md`;
- adding a facade, `core`, `api`, or registry with no real boundary;
- adding a feature dependency on `labonair-shell`;
- bypassing a typed contract to reach private state;
- keeping a transitional edge without a named removal condition.

## Documentation

- copying a canonical table into a second manually maintained document;
- using a report or archive as current authority;
- marking a target statement as proof of implementation;
- marking a feature complete because it compiles;
- leaving stale status, missing owner, or missing evidence unqualified;
- adding a document without linking it from the appropriate canonical index.

## UI and product surfaces

- adding a parallel settings, toast, menu, panel, status, or error surface;
- using a local button/list/menu/input style instead of `labonair-ui-kit`;
- adding shell chrome without an ADR;
- adding a visible UI action that bypasses the owner command contract;
- accepting a screenshot of the legacy Tauri application as native evidence.

## AI, remote, and security

- exposing secrets in logs, tests, SQLite, fixtures, source, or commits;
- implementing an MCP tool without a grant, limit, denial path, and owner;
- treating remote paths or host IDs as trusted input;
- using a broad shell command when a typed capability contract is available;
- reporting only successful automation cases and omitting denied cases.

## Workflow

- skipping the earliest incomplete task in `tasks/rework/`;
- creating a second active queue or private TODO system;
- running only a narrow test and reporting a broad gate as complete;
- hiding an unrelated failing gate instead of recording it;
- rewriting unrelated worktree changes;
- blocking the GPUI foreground thread with I/O.
