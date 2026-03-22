# codex-inspired tui overhaul plan

## objective

Deliver one large-step improvement to the Bee TUI by borrowing the strongest interaction patterns
from the local Codex codebase at `/Users/g/Documents/GitHub/codex`, while keeping the current Bee
event model and backend contracts stable.

This pass is explicitly not a cosmetic retheme. It should improve:

- layout clarity,
- activity awareness,
- input confidence,
- operational visibility,
- perceived product maturity.

## codex reference takeaways

Based on the local Codex docs and TUI source:

1. keep the palette restrained
   - primary text stays mostly default/light
   - secondary context is dim rather than saturated
   - color is reserved for state, not decoration

2. give the composer structural authority
   - composer owns guidance, mode hints, and status-adjacent context
   - footer text should explain the next safe action

3. surface context without clutter
   - show ambient runtime info and active state in compact lines
   - avoid heavy chrome and large framed regions

4. adapt to width
   - wide terminals can afford a side rail
   - narrow terminals should collapse back to a single clear main column

## target experience

### 1. responsive workspace

- narrow mode: status, transcript, composer
- wide mode: transcript + right rail + composer
- the right rail should feel like an operator HUD rather than a second chat column

### 2. operational side rail

The rail should summarize what matters now:

- current phase
- active tool / last tool-like activity
- conversation stats
- recent system/tool events
- compact shortcut/help reference

### 3. stronger transcript rhythm

- preserve the existing clean full-background chat look
- improve message sectioning and scanability
- make assistant output feel like the visual anchor

### 4. composer with codex-like guidance

- keep the simplified input body
- improve footer guidance and state readout
- make the bottom area feel intentional, not just a text box with labels

## implementation scope

### phase a: docs and progress tracking

- create this plan
- keep a progress log in this file

### phase b: new layout + rail

- add a new widget for the right rail
- compute rail content from current `UiState`
- wire responsive split logic into `src/ui/render.rs`

### phase c: transcript/composer polish

- tune transcript spacing, section labels, and secondary text
- rework composer footer and context hierarchy
- keep slash popup visually aligned with the new structure

### phase d: verification

- run `cargo check`
- run focused widget tests where applicable
- update progress log with outcomes

## acceptance criteria

- wide terminals show a meaningful right rail
- narrow terminals still render cleanly with no broken layout
- no new backend API is required
- no logging leaks back into the TUI
- the result feels like a materially more capable tool, not a small style tweak

## progress log

- [x] plan created
- [x] inspect Codex-inspired structures and map them to Bee
- [x] implement responsive layout and side rail
- [x] refine transcript and composer hierarchy
- [x] verify with `cargo check` and focused tests

## completion notes

- added a responsive right-side activity rail for wide terminals
- upgraded the bottom composer into a codex-style contextual footer stack
- composer now expands with draft height and supports `shift+enter` newline
- removed the last top status row and folded status awareness into the rail/composer
- input now supports mid-buffer cursor editing and local `@file` search popup
- multiline drafts now support vertical cursor movement with preserved column intent
- inserted `@file` references are rendered as distinct inline tokens instead of plain body text
- visual up/down movement now follows wrapped composer rows, not only logical newline rows
- `@file` references now move and delete atomically during cursor navigation/editing
- composer viewport now follows the active cursor instead of always pinning to the tail
- slash and file popups now use fuzzy ranking instead of plain prefix / contains filtering
- kept the transcript as the primary surface while improving operational visibility
- verified with:
  - `cargo check`
  - `cargo test ui::widgets::conversation::tests::`
  - `cargo test ui::widgets::status_indicator::tests::`
  - `cargo test ui::widgets::command_popup::tests::`
  - `cargo test ui::widgets::file_popup::tests::`

## implementation notes

- prefer additive widgets and render helpers over large monolithic rendering logic
- avoid introducing custom terminal colors that overpower default terminal themes
- preserve the current command history and slash-command interaction model
