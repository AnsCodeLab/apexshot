---
name: GTK History sizing and focus
description: Why the History sidebar width resisted changes, what actually controls it, and how selected-looking states arise
---

## Sidebar width: only the wrapper's CSS `min-width` works

A `ScrolledWindow` whose horizontal policy is `Never` measures its child directly and
**ignores `min-content-width` / `max-content-width`**. Setting those is dead code there.
Widget `set_size_request` is also only a minimum, not an exact allocation.

So the History sidebar's width is governed by exactly one lever: the CSS `min-width` on
`.history-root .settings-sidebar-wrapper`, combined with `hexpand(false)` so it cannot
grow past it.

**Why:** Several rounds of width edits appeared to do nothing, then overshot into a
too-narrow sidebar. Measuring the user's screenshots (normalising scale by comparing a word
present in both windows, plus nav-row pitch) showed the sidebar collapsing to its content's
natural width (~128px) the moment the CSS minimum was set to 0, while scroller content-width
pins had no effect at any value.

**How to apply:** To change the History sidebar width, edit that CSS `min-width` only. Do not
reach for scroller content-width pins or widget size requests; they will silently do nothing.

## Settings' sidebar is wide because of hexpand propagation, not its own width

Settings' sidebar renders far wider than its 170px CSS floor (~290px at the default 1020px
window) because its Save button sets `hexpand(true)`. In GTK4 hexpand propagates **up** from
a child, so the whole sidebar wrapper becomes an expanding child of the content split and
takes a share of the window's leftover space.

**Why:** "Match the Settings sidebar width" is therefore not "use Settings' 170px CSS value" —
that produces a visibly narrower sidebar and a mismatch the user will notice immediately.
Replicating the mechanism does not give parity either, because the share depends on the
opposite column's minimum width, which differs between the two windows.

**How to apply:** When matching a sidebar to Settings, match its *rendered* width, and check
whether an hexpanding descendant (buttons set to fill) is inflating the reference.

## Focus vs. selection

Popover focus and hover are separate concerns. Opening a popover underneath the pointer can
make its first row look selected even when focus styling is correct, while ordinary mouse
focus can leave search fields and cards looking selected after interaction. Persistent
"which page am I on" indicators, however, are real selection and must survive focus moving
elsewhere.

**Why:** Styling generic `:focus` as selection produced both stale highlights on transient
controls and, when over-corrected, the loss of the sidebar's page indicator.

**How to apply:** Clear focus on primary clicks for transient controls (search, cards), keep
keyboard-visible focus via `:focus-visible`, drive page indicators with an explicit
`-selected` class synced to the stack, and anchor pointer-opened menus so no row starts hovered.
