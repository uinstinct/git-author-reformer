# Quick Task 260524-kgx: Add author selection list to rename form - Research

**Researched:** 2026-05-24
**Domain:** ratatui TUI state machine (this codebase only)
**Confidence:** HIGH — all findings verified by reading src/tui/{app,event,render}.rs and src/git/types.rs

## Summary

The `Screen::RenameForm` variant currently carries only `{ source, draft }`. Adding a third
focus target (an embedded, filterable author list mirroring `Screen::AuthorList`) requires:
(1) extending `FormField` from 2 to 3 variants with a 3-way `toggle()`; (2) widening the
`RenameForm` variant with nucleo/filter/matched/selected fields (built at AuthorList→RenameForm
transition, excluding `source`); (3) branching the `Char`/`Backspace`/`Enter`/`Up`/`Down` key
arms on focus; (4) adding a third layout zone + a third cursor branch in `render_rename_form`.

All helper plumbing already exists and is reusable as-is — `build_author_nucleo`/`apply_filter`
operate on any `&[AuthorIdentity]`. No new dependencies. No security surface. The work is a
pure refactor of three existing files plus their inline test modules.

**Primary recommendation:** Make `toggle()` a fixed 3-way cycle (Name→Email→List→Name). When the
List is focused but empty, `Enter`/`Up`/`Down` are no-ops; the form stays usable via Name/Email.
Update the breaking tests as a Wave 0 task before implementation.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Selecting an author **autofills** `new_name` + `new_email` (still editable afterward). Not a mode toggle — a convenience populating the existing draft fields.
- List is filterable like the first screen (fuzzy via nucleo, filter updates as you type).
- `Tab`/`BackTab` cycles **three** targets: Name, Email, List.
- Typing routes to the focused target (chars edit Name/Email text, or update list filter when List focused).
- When List focused: `Up`/`Down` move selection, `Enter` autofills from highlighted author. `Enter` on a text field still submits when complete (preserve today's behavior).
- List contents = `enumerate_authors` result **excluding** `source` (match on full `AuthorIdentity` — name AND email).
- Display format mirrors `render_author_list`: `{commit_count:>4}  {name} <{email}>`, "/ {filter}" row, "Authors (N match)" title.

### Claude's Discretion
- Exact ratatui layout proportions (header + name + email + filter + list + footer).
- Visual focus indication for list vs. text fields (reuse existing bold/highlight conventions).

### Deferred Ideas (OUT OF SCOPE)
- None recorded.
</user_constraints>

<phase_requirements>
## Phase Requirements
Quick task — no formal requirement IDs. Behavior fully specified in User Constraints above.
</phase_requirements>

## 1. Focus model extension (`app.rs:91-104`)

**Change:** Add a `List` variant; make `toggle()` a 3-way cycle.

```rust
pub enum FormField { Name, Email, List }   // app.rs:92

impl FormField {
    pub fn toggle(self) -> Self {           // app.rs:98
        match self {
            Self::Name => Self::Email,
            Self::Email => Self::List,
            Self::List => Self::Name,
        }
    }
}
```

- `RenameDraft::is_complete()` (`app.rs:86`) is **unaffected** — it only reads `new_name`/`new_email`, not `focused`. No change needed. `[VERIFIED: app.rs:86-88]`
- `RenameDraft::default()` (`app.rs:80`) keeps `focused: FormField::Name`. No change.
- `BackTab` currently aliases `Tab` (`event.rs:203`). A 3-way forward-only `toggle()` means `BackTab` cycles forward too. CONTEXT says BackTab should cycle — if true reverse cycling is wanted, add a `toggle_back()`; otherwise leave the alias (recommend leaving it for simplicity, matching today's behavior where BackTab == Tab).

## 2. Nucleo ownership inside `RenameForm` (`app.rs:24-27`)

**Widen the variant** to mirror `AuthorList`:

```rust
RenameForm {
    source: AuthorIdentity,
    draft: RenameDraft,
    items: Vec<AuthorIdentity>,
    filter: String,
    matched: Vec<AuthorIdentity>,
    nucleo: Nucleo<AuthorIdentity>,
    selected: usize,
},
```

- `build_author_nucleo` / `apply_filter` are reusable **as-is** — both take `&[AuthorIdentity]` / `&mut Nucleo<AuthorIdentity>`. No new helpers needed. `[VERIFIED: app.rs:159-179]`
- `Nucleo<AuthorIdentity>` is **not `Clone`**, but it is never cloned in existing code — stored by value, accessed by `&mut`. **No borrow-checker consequences** as long as the new match arms destructure by reference (`&mut app.screen` already in `handle_key`, and render destructures `&app.screen`). The existing `AuthorList` arm is the exact template (`event.rs:167-200`, `render.rs:13-18`). `[VERIFIED: event.rs:171, render.rs:16]`
- The NLL clone-before-reassign pattern at `event.rs:208-213` (clone `source` out before assigning `app.screen`) stays valid; widening the variant doesn't affect it.

## 3. Excluding the source author (`event.rs:181-188`)

The transition lives in the `Screen::AuthorList` Enter arm. Two changes:

1. **`event.rs:168`**: the arm currently binds `items: _`. Change to `items` so it's in scope.
2. Build the excluded list before constructing `RenameForm`:

```rust
KeyCode::Enter => {
    if let Some(src) = matched.get(*selected).cloned() {
        let rest: Vec<AuthorIdentity> =
            items.iter().filter(|a| **a != src).cloned().collect();
        let mut nucleo = build_author_nucleo(&rest);
        let matched_list = apply_filter(&mut nucleo, "");
        app.screen = Screen::RenameForm {
            source: src,
            draft: RenameDraft::default(),
            items: rest,
            filter: String::new(),
            matched: matched_list,
            nucleo,
            selected: 0,
        };
    }
}
```

- `AuthorIdentity` derives `PartialEq, Eq` (`[VERIFIED: src/git/types.rs:1]`), so `**a != src`
  matches on **both name and email** — exactly the CONTEXT requirement. No manual field compare needed.

## 4. Key-routing collisions (`event.rs:201-240`, the `Screen::RenameForm` arm)

Match arm must destructure the new fields, then branch on `draft.focused`:

| Key | Name/Email focused | List focused |
|-----|--------------------|--------------|
| `Char(c)` (`event.rs:235`) | push to focused text field (today) | `filter.push(c)`; `*matched = apply_filter(nucleo, filter)`; `*selected = 0` (mirror `event.rs:194-198`) |
| `Backspace` (`event.rs:227`) | pop focused text field (today) | `filter.pop()`; reapply filter; reset selected (mirror `event.rs:189-193`) |
| `Enter` (`event.rs:207`) | submit when `is_complete()` (today) | autofill: `if let Some(a) = matched.get(*selected) { draft.new_name = a.name.clone(); draft.new_email = a.email.clone(); }` — **do NOT submit, do NOT change screen** |
| `Up`/`Down` | (none today) | move selection when `!matched.is_empty()` (mirror `event.rs:175-179`) |
| `Tab`/`BackTab` (`event.rs:203`) | unchanged — `draft.focused = draft.focused.clone().toggle()` | same |
| `Esc` (`event.rs:202`) | unchanged → MainMenu | same |

**Empty-list edge case (the one genuine ambiguity — CONTEXT doesn't fully resolve it):**
CONTEXT says "do not block submission or crash on empty list" but not whether Tab cycles *into*
an empty List or skips it. Two valid readings:
- **(a) Fixed 3-way cycle.** Tab always reaches List; when `matched.is_empty()`, `Enter`/`Up`/`Down`
  are no-ops (the `matched.get(*selected)` returns `None`; guard Up/Down with `!matched.is_empty()`).
  Simpler — `toggle()` has no branching.
- **(b) Dynamic cycle.** Skip List in `toggle()` when the list is empty. Adds state-dependent
  branching to focus logic.

**Recommend (a)** for simplicity. Planner should lock the choice. Either way, `is_complete()`-gated
submission via Name/Email keeps the form usable when the list is empty.

## 5. Render changes (`render.rs:144-214`, `render_rename_form`)

- **Layout (`render.rs:145-151`)**: add two zones — a filter row and the list body. Discretionary
  proportions. Suggested vertical stack: header `Length(3)`, name `Length(3)`, email `Length(3)`,
  filter `Length(3)`, list `Fill(1)`, footer `Length(1)`. **Pitfall:** the four fixed `Length(3)`
  blocks total 12 rows + footer; on a short terminal the `Fill(1)` list collapses to zero rows.
  Acceptable (list just shows empty) but worth a glance — the existing `render_hook_add_list`
  (`render.rs:386-392`) already stacks 4+1 zones successfully, so this is a proven pattern.
- **List body**: copy the `ListItem`/`List`/`ListState` block from `render_author_list:117-136`
  verbatim (same `{:>4}  {} <{}>` format, same `"Authors (N match)"` title, same empty→`None`
  selection guard).
- **Cursor (`render.rs:200-208`)**: currently a 2-branch if/else (Name vs Email). Add a **third
  branch** for `FormField::List` placing the cursor on the filter row — mirror `render_author_list`'s
  `cursor_x = filter_row.x + 1 + 2 + filter.chars().count()` / `cursor_y = filter_row.y + 1`
  (`render.rs:112-114`).
- **Signature**: `render_rename_form` (`render.rs:144`) and its call site (`render.rs:19-21`) must
  pass `filter`, `matched`, `selected` (the render dispatch already destructures `Screen::RenameForm`
  by ref — extend it with `..` or the new fields).
- Footer hint (`render.rs:210-213`): update to mention list nav, e.g. `"Tab: switch  type/Up/Down on list: pick  Enter: confirm/autofill  Esc: cancel"`.

## 6. Test breakage (Wave 0 — fix before / alongside implementation)

These WILL fail to compile or assert once the variant/enum change lands:

**`src/tui/app.rs`:**
- `app.rs:273` `test_form_field_toggle` — asserts 2-way `Name<->Email`. **Rewrite** to assert
  `Name→Email→List→Name`.

**`src/tui/event.rs`** — every `Screen::RenameForm { source, draft }` struct literal and the one
destructure must add the new fields (or be built via a helper). Affected sites:
- `event.rs:630` (destructure in `test_author_list_enter_transitions_to_rename_form_with_selected_source`)
- `event.rs:651, 675, 699, 740, 762` (struct-literal constructions in tests)
- `event.rs:648` `test_rename_form_tab_toggles_focused_field` — second `Tab` currently expects
  `Name`; with 3-way cycle it lands on **`List`**. **Update** the second assertion (and ideally add
  a third Tab → `Name`).

**Recommendation:** add a test helper `make_rename_form_screen(source, others: &[...])` mirroring
`make_author_list_screen` (`event.rs:469-487`) to construct the widened variant in one place, so
future field additions touch one function. Tests that build `RenameForm` inline with only
`{ source, draft }` should switch to it.

New tests to add (encode intent per CLAUDE.md Rule 8):
- Tab from Email focuses List; Tab from List focuses Name.
- With List focused + non-empty matched, `Enter` autofills name+email from highlighted author and
  **stays on RenameForm** (does not transition to Preview).
- With List focused, `Char`/`Backspace` update `filter` + `matched` (not the text fields).
- Empty excluded-list: List focusable, `Enter` is a no-op, form still submittable via Name/Email.
- Source author is absent from the built `items`/`matched`.

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---------|-------------|-----|
| Fuzzy filtering the embedded list | `build_author_nucleo` + `apply_filter` (`app.rs:159,172`) | Already the exact engine `AuthorList` uses; reuse verbatim |
| Excluding source | `items.iter().filter(\|a\| **a != src)` | `AuthorIdentity: Eq` derived (`types.rs:1`) — full identity compare for free |
| List rendering | Copy `render_author_list` body (`render.rs:117-136`) | Identical format/title/highlight already specified by CONTEXT |

## Common Pitfalls

1. **Forgetting `items: _` → `items` at `event.rs:168`.** The exclusion filter needs the full author
   list in scope at the transition. Compile error if missed; silent wrong behavior if you instead
   rebuild from `matched` (which is filtered by the *previous* screen's query).
2. **`Enter` on focused List submitting the form.** Must branch: List-focused Enter autofills only;
   Name/Email-focused Enter keeps today's `is_complete()`-gated submit (`event.rs:207`). Crossing
   these is the highest-risk regression.
3. **Cursor on the wrong zone when List focused.** The 2-branch cursor block (`render.rs:200-208`)
   silently places the cursor in the email field unless a third branch is added.
4. **Layout starvation on short terminals.** Four `Length(3)` zones + footer can squeeze the
   `Fill(1)` list to zero height. Not a crash (ratatui clamps), but verify on an 80x24 terminal.
5. **Test struct-literal breakage is compile-time, broad.** ~6 sites in event.rs + 1 in app.rs.
   A test helper contains the blast radius.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | BackTab should keep aliasing Tab (forward cycle) rather than reverse-cycle | §1 | Minor UX; user wanted true reverse. Cheap to add `toggle_back()` later. |
| A2 | Empty-list policy = fixed 3-way cycle with no-op Enter (reading (a)) | §4 | Planner should confirm; reading (b) is also valid per CONTEXT. |

## Sources

### Primary (HIGH — read this session)
- `src/tui/app.rs` — `FormField`, `RenameDraft`, `Screen::RenameForm`, `build_author_nucleo`, `apply_filter`, inline tests
- `src/tui/event.rs` — `handle_key` AuthorList + RenameForm arms, transition site, inline tests
- `src/tui/render.rs` — `render_author_list`, `render_rename_form`, render dispatch
- `src/git/types.rs:1` — `AuthorIdentity` derives `PartialEq, Eq`

## Metadata

**Confidence:** HIGH across the board — change is confined to three files whose full contents were read; all reuse patterns (nucleo helpers, list rendering, NLL clone-before-reassign) already exist in-repo as working templates.
**Research date:** 2026-05-24
**Valid until:** stable (no external deps; valid until these files are restructured)
