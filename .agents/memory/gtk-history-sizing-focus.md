---
name: GTK History sizing and focus
description: Non-obvious GTK behavior behind fixed sidebar widths and stale selected-looking controls
---

GTK widget size requests are minimums, not exact allocation constraints. A scroller that
propagates its child's natural width can therefore remain wider even when the wrapper has
a smaller width request.

**Why:** Repeated CSS minimum-width and widget size-request changes did not visibly shrink
the History sidebar because the scroller continued propagating the navigation content's
wider natural size.

**How to apply:** For a fixed History sidebar, disable natural-width propagation, constrain
the scroller's minimum and maximum content widths to the same value, remove inherited CSS
minimums, and keep expansion on the content column rather than the sidebar.

Popover focus and hover are separate concerns. Opening a popover underneath the pointer
can make its first row look selected even when focus styling is correct, while ordinary
mouse focus can leave search fields, links, and cards looking selected after interaction.

**Why:** Styling generic `:focus` as selection and positioning a menu body beneath the
invoking pointer both produced persistent or immediate selected-looking states.

**How to apply:** Transfer or clear focus on primary clicks, preserve keyboard-visible
focus, and give pointer-opened popovers an exclusion gap so no action row begins hovered.