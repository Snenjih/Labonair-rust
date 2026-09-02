# License Audit

Audit of Labonair-rust's own license and its full dependency tree, run for the
T15-004 release. Re-run when `Cargo.lock` changes materially.

## This project

MIT (`/LICENSE`). Distributing a compiled binary is unrestricted.

## Method

Parsed every `[[package]]` in `Cargo.lock` and read the `license` field of each
crate's published `Cargo.toml` in the local cargo registry. ~1000 crate
versions in the graph (incl. dev-only / build-only / platform-gated crates that
never ship in the macOS/Linux release binary).

## Result — clear

No crate in the tree is licensed **only** under a strong copyleft license
(GPL / AGPL / LGPL-without-alternative). Every dependency is distributable
under a permissive license (MIT / Apache-2.0 / BSD / ISC / Zlib / Unicode-3.0
/ MPL-2.0), and for any crate offering a choice we take the permissive branch.

### GPUI (the historical concern)

`gpui 0.2.2` and all its `gpui_*` companion crates
(`gpui_media`, `gpui_perf`, `gpui-macros`, `gpui_refineable`, …) are
**`Apache-2.0`**. Earlier iterations of Zed's GPUI were GPL-3.0; the crates.io
0.2.x line used for this port is not. No copyleft obligation from the UI
framework.

### Dual/multi-license crates offering a copyleft option (we pick permissive)

| Crate | License expression | Our choice |
|---|---|---|
| `self_cell` | `Apache-2.0 OR GPL-2.0-only` | Apache-2.0 |
| `r-efi` (build/UEFI, not linked on macOS/Linux) | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | MIT/Apache-2.0 |

### Weak copyleft (file-level, MPL-2.0) — compliant as-is

| Crate | Role | Notes |
|---|---|---|
| `option-ext` | transitive dep of `dirs` | MPL-2.0 is file-level; the unmodified binary imposes no obligation beyond making that file's source available (it is, on crates.io / GitHub). |
| `dwrote` | Windows DirectWrite text backend | **Not compiled** — Windows is not a target. |
| `cbindgen` | build-time header generator (transitive) | Build tool, not linked into or distributed with the binary. |

### Other non-MIT/Apache permissive licenses present (all OK to redistribute)

- **Unicode-3.0** — the `icu_*` / `zerovec` / `tinystr` / `writeable` / `yoke`
  family (Unicode data + zero-copy libs). Permissive, attribution only.
- **BSD-2/3-Clause, ISC, Zlib, 0BSD, BSL-1.0, CC0-1.0, Unlicense** — scattered
  small crates, all permissive.
- **CDLA-Permissive-2.0** — `webpki-roots` (Mozilla CA bundle). Permissive.
- **NCSA** — `libfuzzer-sys`, dev-only (fuzz targets), never shipped.

## Distribution obligations

When shipping a binary, include attribution / license text for the bundled
third-party code (standard for MIT/Apache/BSD). Generate a bundled
`THIRD-PARTY-LICENSES.txt` at release time, e.g.:

```sh
cargo install cargo-about   # one-time
cargo about generate about.hbs > THIRD-PARTY-LICENSES.txt
```

(Not committed here — regenerated per release and attached to the GitHub
release / placed in the app bundle's `Resources/`.)

## Bundled non-code assets

- **Fonts** — Inter + JetBrains Mono, both **SIL OFL 1.1**
  (`crates/theme/assets/fonts/LICENSE`). OFL permits bundling in software.
- **App icon** — project asset (`packaging/macos/AppIcon.icns`), copied from
  the frozen `reference-src/` design reference.
