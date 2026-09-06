# Native Visual Verification

**Status:** Normative

Visual checks must prove that the native Rust application was rendered. A
successful process launch or a screenshot of a window with the display name
`Labonair` is not sufficient because the legacy Tauri application may be
installed under the same name.

## Canonical launch target

Use one of these explicit targets:

```text
cargo run -p labonair
target/release/bundle/macos/Labonair.app
```

Never use `open -a Labonair`. For a packaged macOS check, open the absolute
bundle path with `open -n -W` and identify the resulting process by its exact
executable path:

```text
target/release/bundle/macos/Labonair.app/Contents/MacOS/labonair
```

The native bundle uses `com.labonair.rust`; the legacy Tauri application uses
a different bundle identity. A visual helper must fail closed when it cannot
find a layer-0 window owned by the exact Rust PID. It must never fall back to
the generic application name or to the frontmost unrelated window.

## Required evidence

For a shell or layout change, verify the relevant normal, narrow, focused,
empty, loading, and error states. Record the absolute screenshot path and the
Rust executable PID in the task handoff. A process-only smoke test proves
startup and survival, but it does not replace a screenshot or equivalent
rendered visual evidence.

If macOS Screen Recording or Accessibility permission prevents interaction or
capture, record that limitation explicitly and leave the visual acceptance
criterion open. Do not accept a screenshot produced without exact process
ownership, and do not infer visual success from a successful build.

## Repository helpers

- `scripts/package-macos.sh` builds the canonical bundle.
- `scripts/smoke-test.sh` verifies the bundle, core smoke tests, and optionally
  the exact Rust process with `LABONAIR_SMOKE_LAUNCH=1`.
- `scripts/screenshot.sh` accepts an explicit Rust PID and fails closed when
  no matching native window can be captured.

These helpers are verification tools, not additional application behavior.
