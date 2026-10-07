---
tags: [ui, home]
---
# UI Review: start here

**What we're doing:** the app's features work, but the screens need a redesign. We're redesigning them **one page at a time**, as clickable **mocks** built with **Mantine** and fake data, until the product owner approves each one. Mocks are for finding out what works. They don't touch the real app or the Rust code. When pages are approved, they become the brief for rebuilding the real screens.

**Why:** today's screens were hand-built piece by piece, so they're inconsistent, cluttered (e.g. 17 buttons at the bottom of the till), and hard to navigate. We now use one battle-tested component library (Mantine) and the layouts every major POS uses.

## For Claude in a new chat: read in this order
1. [[UI Review Process]]: where mocks live, the review loop, statuses, and what to do when a page is approved.
2. [[UI Guidelines]]: the rules every page follows (components, theme, layouts, patterns, QoL checklist, don'ts). **Consistency comes from here.**
3. [[Design System]]: colours, colour meanings, contrast and tooltip rules (unchanged).
4. [[Page List]]: every page in review order with its status.
5. The spec of the one page you're asked to work on, in `Pages/` (e.g. [[P05 Checkout]]).

Also follow the project rules in `CLAUDE.md` (plain English, no shadows or dark mode, all text kept in one place) and [[Writing Style]].

**Work on one page per session.** If the owner names no page, take the first page in [[Page List]] that isn't approved. If `ui-mocks/` doesn't exist yet, set it up first (see [[UI Review Process#Where the mocks live]]) together with **P00**.

## For the product owner
Copy one of these into a new chat:

**The very first time:**
> Read `docs/13 UI Review/UI Review Home.md` and follow it. Set up the `ui-mocks` playground, then build the mock for page **P00 App shell & navigation**. Show me screenshots and the link, then wait for my review.

**For each next page:**
> Read `docs/13 UI Review/UI Review Home.md` and follow it. Build the mock for page **P05 Checkout**. Show me screenshots and the link, then wait for my review.

**When a page is right:**
> Approved. Record the decisions, update the guidelines and the page list, and commit.

Use **Opus 5.5, medium effort**. Use **high** for P05 Checkout, P09 Payment and P13 Returns, which have the most to get right.

## Decision
[[ADR-024 Mantine for the UI]]: Mantine replaces hand-built components. Status: *proposed*, confirmed when P00 is approved.
