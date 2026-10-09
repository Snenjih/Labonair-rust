# Platform Policy

**Status:** Normative
**Version:** 1
**Related:** [`README.md`](README.md), [`../RELEASE.md`](../RELEASE.md), [`../visual-verification.md`](../visual-verification.md)

- macOS Apple Silicon is the primary packaged target.
- macOS Intel is supported through an explicit cross-target build.
- Linux is build-prepared but not a first-class packaged release until its
  AppImage/Flatpak decision and smoke evidence are accepted.
- Windows is outside the current supported product scope.

Platform claims require a native build, launch/smoke result, and support
decision. A successful Linux container build does not close macOS visual or
packaging evidence.
