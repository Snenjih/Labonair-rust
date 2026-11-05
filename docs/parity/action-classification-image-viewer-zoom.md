# Image Viewer Zoom Action Classification

**Status:** Static action, input, and view-state requirements; native dispatch,
toolbar behavior, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Planned Preview capability owns image rendering, zoom state,
actions, and toolbar. Editor/Workspace hosts the view; Keymap owns bindings.
Labonair has no canonical Preview crate assigned yet.

## Evidence boundary

This packet independently states the Image Viewer zoom workflows and current
Labonair gap. The static default action inventory and source review establish
the key contexts, input effects, scale rules, toolbar, and item persistence;
they do not establish native dispatch, rendered behavior, or parity. Work in
this session is independent redevelopment, not separated-room clean-room
implementation, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Native macOS runtime and paired capture access are unavailable in this
session. Action rows remain pending for runtime acceptance.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Planned Preview owns image rendering, per-view zoom/pan, image controls, and zoom action handlers. Editor/Workspace opens and hosts the view; Keymap owns the `ImageViewer` context bindings. A canonical Preview crate still needs an explicit architecture decision before implementation. |
| Active context | All five keyboard actions are scoped to `ImageViewer`; verify that focus is inside the active image view and that actions do not leak into Editor text or unrelated tabs. The toolbar appears only while an ImageView is the active pane item. |
| Keyboard entry | [`ACT-MACOS-image_viewer-ZoomIn`](action-requirements.md) uses `cmd-=`/`cmd-+` on macOS and `ctrl-=`/`ctrl-+` on Linux/Windows. [`ZoomOut`](action-requirements.md) uses `cmd--`/`ctrl--`. [`ResetZoom`](action-requirements.md) uses `cmd-0`/`ctrl-0`; [`ZoomToActualSize`](action-requirements.md) uses `cmd-1`/`ctrl-1`; [`FitToView`](action-requirements.md) uses `cmd-shift-0`/`ctrl-shift-0`. All are no-argument actions in the `ImageViewer` context. |
| Toolbar entry | The active Image Viewer toolbar has Zoom Out, the current percentage, Zoom In, and Fit to View. Click the percentage to edit it; right-click it to reset to 100%. No ZoomToActualSize toolbar control was found. |
| Scale and fit | Zoom In multiplies by 1.1; Zoom Out divides by 1.1. Action, wheel, pinch, and percent-entry updates clamp to 10%–2000%. `FitToView` uses the smaller width/height ratio and caps it at 100%, preserving aspect ratio without upscaling. A very large image in a tiny view may fit below 10% because FitToView computes its ratio separately from the clamped adjustment path; verify this edge. The first layout also starts at fit-to-view scale. |
| Reset and actual size | `ResetZoom` and `ZoomToActualSize` have the same visible effect: set 100% and clear pan offset. They remain separate actions with different default shortcuts; ResetZoom also has the percentage label's right-click route. FitToView is not a 100% reset and may produce a smaller scale. |
| Pointer and gesture input | Control or the platform modifier plus wheel zooms around the pointer position; positive delta increases and negative delta decreases. Pixel delta is used directly and line delta is converted at 20 px per line before a 1%-per-unit scale factor. Unmodified wheel input pans the image. Pinch zooms around the gesture position and is clamped by the same adjustment path. Left- or middle-button drag pans with open-hand/closed-hand cursor feedback. Verify modifier meaning per platform and that input remains inside the active image surface. |
| Percentage editing | The displayed integer percentage can be edited. Keep only up to four ASCII digits; commit on blur or Confirm; cancel on invalid/zero input or Cancel. Committed positive values convert percent to scale and clamp to 10%–2000%. Preserve the previous scale if editing is canceled. |
| Settings and persistence | Zoom and pan have no Settings value. `ImageViewerSettings.unit` selects binary/decimal image-file size labels and is unrelated to zoom. The serializable ImageView item stores its image path but not zoom/pan state; do not claim zoom persistence across restore. Fit behavior after item restore/new layout needs acceptance. |
| Events and registry contributions | Preview contributes five typed view actions and active-item toolbar controls. Keymap contributes the platform defaults above. No global menu, settings field, generic zoom registry, or Workspace zoom target is needed for image-content scale. |
| Failure and recovery | Zoom/pan do not perform I/O. FitToView has no effect until both container bounds and image dimensions exist. Invalid percent editing cancels without changing the previous view. Image decode/read failures belong to the separate Preview open workflow. |
| UI kit | Build toolbar icons and tooltips on UI-kit buttons, shared labels, and the existing UI-kit input contract. Keep the percentage editor and zoom controls in the Preview owner rather than adding shell chrome. |
| Labonair route | No ImageView/ImageViewer code, preview capability crate, zoom action, or image toolbar route was found. `file.previews` remains Missing or unverified. |

## Action requirements and current route

| Requirement ID | Binding and UI entry | User-visible outcome | Current Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-image_viewer-ZoomIn`](action-requirements.md) | `cmd-=`/`cmd-+` on macOS; `ctrl-=`/`ctrl-+` on Linux/Windows; `ImageViewer`. Toolbar `+`; modifier-wheel and pinch are additional inputs. | Multiply the current scale by 1.1, clamp to 10%–2000%, and preserve current pan unless the pointer gesture supplies an anchor. | No Image Viewer or typed action route exists. |
| [`ACT-MACOS-image_viewer-ZoomOut`](action-requirements.md) | `cmd--` on macOS; `ctrl--` on Linux/Windows; `ImageViewer`. Toolbar `−`; modifier-wheel and pinch are additional inputs. | Divide the current scale by 1.1 and clamp at 10%; preserve the current pan except for anchored pointer zoom. | No Image Viewer or typed action route exists. |
| [`ACT-MACOS-image_viewer-ResetZoom`](action-requirements.md) | `cmd-0` / `ctrl-0`, `ImageViewer`; right-click the percentage label. | Set scale to 100% and pan offset to zero. | No Image Viewer, reset action, or percentage-label control exists. |
| [`ACT-MACOS-image_viewer-ZoomToActualSize`](action-requirements.md) | `cmd-1` / `ctrl-1`, `ImageViewer`; no toolbar button found. | Set scale to 100% and pan offset to zero, matching ResetZoom's effect. | No Image Viewer or actual-size action route exists. |
| [`ACT-MACOS-image_viewer-FitToView`](action-requirements.md) | `cmd-shift-0` / `ctrl-shift-0`, `ImageViewer`; toolbar Fit to View button. | Fit both image dimensions within the viewport, preserve aspect ratio, do not upscale above 100%, and clear pan. | No Image Viewer, fit action, or image toolbar exists. |

## Required user-flow and state acceptance

- Open portrait and landscape images at normal and narrow viewport sizes. Verify
  first-layout fit, no upscaling, visible percentage, preserved aspect ratio,
  and fit again after the view bounds change.
- Zoom repeatedly to 10% and 2000%. Verify 1.1 keyboard/button steps,
  stable clamping, and percentage input clamping; include the tiny-viewport
  FitToView result below 10% if it occurs.
- Compare ResetZoom, ZoomToActualSize, and FitToView. Confirm the first two
  both restore 100% and zero pan, while FitToView may be smaller and does not
  upscale a small image.
- Edit the percentage by click, confirm, blur, Cancel, zero, malformed text,
  and an over-range number. Verify invalid input preserves the old scale and
  valid input is digit-only, at most four digits, and clamps appropriately.
- Use Command+wheel on macOS and Control+wheel on Linux/Windows; also test
  Control+wheel on macOS. Verify the cursor anchor and positive/negative
  direction. Verify unmodified wheel pans, left/middle drag pans, pinch zooms,
  and modifier gestures do not affect unrelated focused views.
- Close and restore the Image Viewer item. Confirm the image path restores,
  zoom/pan does not claim persistence, and the first-layout fit behavior is
  consistent. Confirm image size units do not affect zoom.
- Capture paired native macOS states for first fit, 100%, fit/actual contrast,
  both limits, toolbar percentage edit, invalid edit, wheel zoom/pan, pinch,
  drag pan, narrow toolbar, and item restore. Record builds, OS, window and
  image bounds, input device, scale, pan offset, zoom percentage, and focus.

## Acceptance evidence still required

Static keymaps and source inspection establish bindings, contexts, action
semantics, scale rules, toolbar inputs, and image-path-only serialization.
Verify active-view dispatch, modifier mapping, fit/percentage edge cases,
visible rendering, item restore, and paired macOS captures on the pinned build
and Labonair. No runtime or visual parity claim is made from the action table
or the absence of a Labonair Image Viewer crate.
