---
tags: [ui, process]
---
# UI Review Process

How we redesign the screens: **one page at a time**, as a clickable **mock** with fake data, reviewed by the product owner until it's approved. Mocks are for deciding what works. They are **not** the real app and are never wired to Rust.

## Where the mocks live
- A separate small app in the repository: **`ui-mocks/`** (React + Vite + **Mantine**, fake data only, no Tauri, no Rust).
- Run it (from the repository folder): `pnpm --filter ui-mocks dev`, then open **http://localhost:1430**.
- Structure:
  ```
  ui-mocks/src/
  ├── theme.ts            # the Mantine theme from the UI Guidelines (one place)
  ├── shell/              # back-office frame and till frame (page P00)
  ├── data/               # fake data, same shop as the demo backup (products, customers, sales…)
  ├── mock-tools/         # the "Mock" switcher: page state, role, screen size
  └── pages/P05-checkout/ # one folder per page, named by its ID
  ```
- A **page index** at `/` lists every page from [[Page List]] with its status, so you can click straight to any mock.
- Every mock has a small floating **Mock** button (bottom-left) to switch:
  - **State:** normal · empty · loading · error · lots of data
  - **Signed in as:** owner · cashier · supervisor (to see hidden buttons and approval prompts)
  - **Screen:** 1366×768 · 1920×1080 (and touch size for the till)

## The loop (one page)
1. **Start a fresh session** with the prompt below. Claude reads this page, [[UI Guidelines]], [[Page List]] and the page's own spec in `Pages/`.
2. Claude **builds the mock** for that one page, using only Mantine components and the shared theme and shell, with realistic fake data and all states.
3. Claude **checks it** in its browser pane (screenshots at 1366×768), fixes obvious problems, then sends you the screenshots and the link.
4. **You review** it, in your own browser or in the chat, using the checklist below. Say what works and what doesn't.
5. Claude **changes the mock**, and steps 3–5 repeat.
6. When you say **"approved"**, Claude:
   - writes the decisions into the page's spec (section "Decisions") and sets its status to **Approved**
   - adds any new reusable pattern to [[UI Guidelines]] (with its change-log row), so later pages stay consistent
   - updates the status in [[Page List]]
   - commits the mock and the docs
7. Next page: **a new session**, so context stays small and focused.

## Order
1. **P00 App shell & navigation** first: it sets the frame, theme and menu for everything else.
2. Then the till flow (P01–P15), because cashiers use it all day.
3. Then the back office, by how often it's used (P16 onwards).
You can always pick a different page: just name it.

## Review checklist (for the product owner)
- **First look:** Can you tell what this page is for in 3 seconds? Is the main button obvious?
- **Navigation:** Could a new cashier or manager find their way here and back without help?
- **Colours:** Does every colour mean what the [[Design System#Colour code]] says?
- **Clutter:** Is anything here rarely used and better hidden under "More"?
- **Words:** Is every label clear and plain? Do the tooltips explain enough?
- **States:** Try the Mock button: empty, loading, error, lots of data, cashier vs owner. Do they all make sense?
- **Speed:** For the till: can you do the common job (scan, pay) without the mouse?
- **Fit:** Does it fit at 1366×768? Is anything too small to tap?

## Statuses
| Status | Meaning |
|---|---|
| ⬜ Not started | No mock yet |
| 🟦 In review | A mock exists and is being reviewed |
| 🟨 Changes asked | The owner asked for changes |
| 🟩 Approved | Decisions recorded; ready to be built for real |
| ⏸ Later | Deliberately postponed |

## After the pages are approved
The approved mocks and their spec pages become the brief for **rebuild tasks** (one per area, fresh sessions, like the C tasks). The real screens then use Mantine with the real Rust commands. No Rust code changes for the redesign.

## Prompts for a new chat
**First session (set up + shell):**
> Read `docs/13 UI Review/UI Review Home.md` and follow it. Set up the `ui-mocks` playground as described in UI Review Process, then build the mock for page **P00 App shell & navigation**. Show me screenshots and the link, then wait for my review.

**Every next page:**
> Read `docs/13 UI Review/UI Review Home.md` and follow it. Build the mock for page **P05 Checkout** (or: "the next page that isn't approved"). Show me screenshots and the link, then wait for my review.

**When you like it:**
> Approved. Record the decisions, update the guidelines and the page list, and commit.
