---
phase: quick-260524-kgx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/tui/app.rs
  - src/tui/event.rs
  - src/tui/render.rs
autonomous: true
requirements: []

must_haves:
  truths:
    - "The source author being renamed is absent from the embedded author list"
    - "Selecting an author from the list autofills New name and New email (both still editable afterward)"
    - "Tab/BackTab cycles focus Name -> Email -> List -> Name"
    - "When the List is focused, typing edits the list filter (not the text fields)"
    - "When the List is focused, Enter autofills from the highlighted author and stays on RenameForm (does not submit)"
    - "When a text field is focused, Enter still submits the form when complete (today's behavior preserved)"
    - "Form remains usable when the list is empty: Tab reaches List, Enter/Up/Down are no-ops, Name/Email submit still works"
  artifacts:
    - path: "src/tui/app.rs"
      provides: "FormField 3-way toggle (Name->Email->List->Name) and widened Screen::RenameForm variant"
      contains: "FormField"
    - path: "src/tui/event.rs"
      provides: "3-way key routing in RenameForm arm + source exclusion at AuthorList->RenameForm transition"
      contains: "RenameForm"
    - path: "src/tui/render.rs"
      provides: "render_rename_form with filter row + list body + third cursor branch"
      contains: "render_rename_form"
  key_links:
    - from: "src/tui/render.rs render() dispatch"
      to: "Screen::RenameForm"
      via: "destructure widened variant (filter, matched, selected)"
      pattern: "Screen::RenameForm"
    - from: "src/tui/event.rs RenameForm arm"
      to: "draft.focused == FormField::List"
      via: "branch Char/Backspace/Enter/Up/Down on focus"
      pattern: "FormField::List"
    - from: "src/tui/event.rs AuthorList Enter arm"
      to: "excluded items list"
      via: "items.iter().filter(|a| **a != src)"
      pattern: "filter"
---

<objective>
Add a filterable author selection list as a third element of the Rename form screen (`Screen::RenameForm`). The list mirrors the first screen (`Screen::AuthorList`): fuzzy-filtered via nucleo, displaying every repo author EXCEPT the one being renamed (the `source`). Selecting an author autofills the New name and New email fields (still editable). Tab cycles focus across three targets: Name, Email, List.

Purpose: Renaming an author to match another existing author (deduplication) is a common task; today the user must retype the target identity by hand. The list makes it a single keystroke while preserving full edit freedom.

Output: Modified `src/tui/app.rs`, `src/tui/event.rs`, `src/tui/render.rs` with updated inline tests.
</objective>

<execution_context>
@/Users/instinct/Desktop/working/git-author-reformer/.claude/get-shit-done/workflows/execute-plan.md
@/Users/instinct/Desktop/working/git-author-reformer/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/quick/260524-kgx-add-author-selection-list-to-rename-form/260524-kgx-CONTEXT.md
@.planning/quick/260524-kgx-add-author-selection-list-to-rename-form/260524-kgx-RESEARCH.md

<interfaces>
<!-- Reusable helpers and types the executor needs. All already exist in-repo. -->

From src/tui/app.rs:
```rust
pub fn build_author_nucleo(items: &[AuthorIdentity]) -> Nucleo<AuthorIdentity>;
pub fn apply_filter(nucleo: &mut Nucleo<AuthorIdentity>, query: &str) -> Vec<AuthorIdentity>;

pub struct RenameDraft {
    pub new_name: String,
    pub new_email: String,
    pub focused: FormField,
}
impl RenameDraft { pub fn is_complete(&self) -> bool; }  // reads only name/email — UNCHANGED

// Current 2-way enum to extend:
pub enum FormField { Name, Email }
```

From src/git/types.rs:
```rust
// AuthorIdentity derives PartialEq, Eq — `**a != src` matches on name AND email.
pub struct AuthorIdentity { pub name: String, pub email: String, pub commit_count: usize }
```

Reference templates to mirror (do NOT modify):
- `Screen::AuthorList` arm in event.rs:167-200 — filter/nav/Enter handling
- `render_author_list` in render.rs:91-142 — list + filter row rendering, cursor math
</interfaces>

<locked_decisions>
From CONTEXT.md (NON-NEGOTIABLE):
- Autofill-on-select: selecting populates new_name + new_email, both still editable. NOT a mode toggle.
- List filterable like screen 1 (fuzzy via nucleo as you type).
- Tab/BackTab cycles THREE targets: Name, Email, List.
- Typing routes to the focused target.
- List focused: Up/Down move selection, Enter autofills from highlighted author. Enter on a text field still submits when complete.
- List contents = enumerate_authors result EXCLUDING source (full AuthorIdentity match — name AND email).
- Display format mirrors render_author_list exactly: `{commit_count:>4}  {name} <{email}>`, "/ {filter}" row, "Authors (N match)" title.

EMPTY-LIST POLICY (resolved here — researcher flagged as the one open ambiguity):
- LOCKED: reading (a) — fixed 3-way Tab cycle. Tab always reaches List even when empty.
- When `matched.is_empty()`: Enter is a no-op (matched.get returns None), Up/Down are no-ops
  (guarded by `!matched.is_empty()`). `toggle()` has NO empty-list branching.
- Name/Email entry + is_complete()-gated submit keeps the form usable when the list is empty.

BackTab: keep aliasing Tab (forward cycle), matching today's behavior. Do NOT add toggle_back().
</locked_decisions>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Extend state shape, transition, and fix compile-breaking tests (Wave 0)</name>
  <files>src/tui/app.rs, src/tui/event.rs</files>
  <behavior>
    - FormField::Name.toggle() == Email; Email.toggle() == List; List.toggle() == Name (3-way cycle)
    - AuthorList -> RenameForm transition builds an `items` list that EXCLUDES the selected source author (matched on full AuthorIdentity)
    - test_form_field_toggle asserts the full 3-way cycle
    - test_rename_form_tab_toggles_focused_field: second Tab lands on List, third Tab returns to Name
  </behavior>
  <action>
In src/tui/app.rs:
1. Add `List` to the `FormField` enum (app.rs:92): `pub enum FormField { Name, Email, List }`.
2. Rewrite `FormField::toggle()` (app.rs:98) as a fixed 3-way cycle: Name=>Email, Email=>List, List=>Name. NO empty-list branching (per locked empty-list policy reading (a)).
3. Widen the `Screen::RenameForm` variant (app.rs:24-27) to add: `items: Vec<AuthorIdentity>`, `filter: String`, `matched: Vec<AuthorIdentity>`, `nucleo: Nucleo<AuthorIdentity>`, `selected: usize` — mirroring the `AuthorList` variant. Keep `source` and `draft`. RenameDraft and is_complete() are UNCHANGED (is_complete reads only new_name/new_email).
4. Update the inline test `test_form_field_toggle` (app.rs:273) to assert the 3-way cycle: Name->Email, Email->List, List->Name.

In src/tui/event.rs (transition only — key routing is Task 2):
5. In the `Screen::AuthorList` arm, change the binding `items: _` (event.rs:168) to `items` so it is in scope.
6. In that arm's `KeyCode::Enter` handler (event.rs:181-188), before constructing `RenameForm`: build the excluded list `let rest: Vec<AuthorIdentity> = items.iter().filter(|a| **a != src).cloned().collect();` then `let mut nucleo = build_author_nucleo(&rest); let matched_list = apply_filter(&mut nucleo, "");`. Construct `Screen::RenameForm { source: src, draft: RenameDraft::default(), items: rest, filter: String::new(), matched: matched_list, nucleo, selected: 0 }`. AuthorIdentity derives Eq, so `**a != src` excludes the exact name+email identity.
7. Add a test helper next to `make_author_list_screen` (event.rs:469): `fn make_rename_form_screen(source: AuthorIdentity, others: &[&str]) -> Screen` that builds the widened variant (others -> AuthorIdentity list -> build_author_nucleo -> apply_filter -> RenameForm). Use this helper everywhere a RenameForm is constructed inline in tests.
8. Update every test that constructs or destructures `Screen::RenameForm { source, draft }` to the widened variant. Affected sites: destructure at event.rs:630; struct-literal constructions at event.rs:651, 675, 699, 740, 762. Switch inline constructions to `make_rename_form_screen` where practical; destructures can use `..` to ignore new fields.
9. Update `test_rename_form_tab_toggles_focused_field` (event.rs:648): after the first Tab (Name->Email), the second Tab now lands on `FormField::List`; add a third Tab assertion returning to `FormField::Name`.

Touch ONLY app.rs and event.rs. Do not refactor unrelated code. Do not add Co-Authored-By trailers to any commit (project Rule 10).
  </action>
  <verify>
    <automated>cd /Users/instinct/Desktop/working/git-author-reformer && rtk cargo build && rtk cargo test --lib tui::app tui::event</automated>
  </verify>
  <done>Project compiles; FormField is a 3-way cycle; Screen::RenameForm carries items/filter/matched/nucleo/selected; the AuthorList Enter transition excludes the source; all previously-passing app.rs and event.rs tests pass (with toggle/Tab tests updated to the 3-way cycle).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Key routing branch + render the embedded list</name>
  <files>src/tui/event.rs, src/tui/render.rs</files>
  <behavior>
    - List focused + Char/Backspace updates `filter` and re-runs `apply_filter` (text fields untouched); selected resets to 0
    - List focused + Up/Down moves `selected` only when `!matched.is_empty()`
    - List focused + Enter autofills draft.new_name/new_email from highlighted author and STAYS on RenameForm
    - Name/Email focused: Char/Backspace/Enter behave exactly as today (Enter submits when is_complete())
    - render() dispatch passes the widened fields; a third cursor branch handles FormField::List
  </behavior>
  <action>
In src/tui/event.rs, the `Screen::RenameForm` arm (event.rs:201-240):
1. Destructure the widened variant: `Screen::RenameForm { source, draft, filter, matched, nucleo, selected }`.
2. Branch each key on `draft.focused`:
   - `KeyCode::Char(c)`: if List focused -> `filter.push(c); *matched = apply_filter(nucleo, filter); *selected = 0;` (mirror event.rs:194-198). Else (Name/Email) -> push to the focused text field (today's behavior at event.rs:235-238).
   - `KeyCode::Backspace`: if List focused -> `filter.pop(); *matched = apply_filter(nucleo, filter); *selected = 0;` (mirror event.rs:189-193). Else -> pop the focused text field (today's behavior at event.rs:227-234).
   - `KeyCode::Enter`: if List focused -> autofill only: `if let Some(a) = matched.get(*selected) { draft.new_name = a.name.clone(); draft.new_email = a.email.clone(); }` and DO NOT submit, DO NOT change screen. Else -> keep today's `Enter if draft.is_complete()` submit path (event.rs:207-226) unchanged. NOTE: the existing guarded arm `KeyCode::Enter if draft.is_complete()` only fires for text-field focus by design once List Enter is handled first — order the List-focused Enter check so it takes precedence when focused on List.
   - `KeyCode::Up` / `KeyCode::Down`: only meaningful when List focused AND `!matched.is_empty()` -> move `*selected` with wraparound (mirror event.rs:175-179). No-op otherwise.
   - `KeyCode::Tab | KeyCode::BackTab`: unchanged — `draft.focused = draft.focused.clone().toggle()`.
   - `KeyCode::Esc`: unchanged -> MainMenu { selected: 0 }.
   The existing NLL clone-before-reassign in the submit path (event.rs:208-213) stays valid; widening the variant does not affect it.

In src/tui/render.rs:
3. Update the `render()` dispatch arm for `Screen::RenameForm` (render.rs:19-21) to destructure and pass `filter`, `matched`, `selected` in addition to source/draft.
4. Extend `render_rename_form` signature (render.rs:144) to accept `filter: &str, matched: &[AuthorIdentity], selected: usize`.
5. Change the layout (render.rs:145-151) to a 5-zone vertical stack mirroring render_hook_add_list: header Length(3), name Length(3), email Length(3), filter Length(3), list Fill(1), footer Length(1). (Layout proportions are Claude's discretion per CONTEXT.)
6. Render the filter row using the same "/ {filter}" + "Filter" bordered block as render_author_list (render.rs:106-110).
7. Render the list body by copying the ListItem/List/ListState block from render_author_list (render.rs:117-136) verbatim: `{:>4}  {} <{}>` format, "Authors (N match)" title, empty->None selection guard.
8. Cursor (render.rs:200-208): convert the 2-branch if/else into a 3-way match on draft.focused. Name -> name field cursor (today). Email -> email field cursor (today). List -> filter-row cursor using `filter_row.x + 1 + 2 + filter.chars().count()` / `filter_row.y + 1` (mirror render_author_list render.rs:112-114).
9. Update the footer hint (render.rs:210-213) to mention list nav, e.g. "Tab: switch  type/Up/Down on list: pick  Enter: confirm/autofill  Esc: cancel".

Touch ONLY event.rs and render.rs. Match existing style. No Co-Authored-By trailers (Rule 10).
  </action>
  <verify>
    <automated>cd /Users/instinct/Desktop/working/git-author-reformer && rtk cargo build && rtk cargo test --lib tui</automated>
  </verify>
  <done>Project compiles; all existing tui tests pass; render_rename_form draws header/name/email/filter/list/footer with a third cursor branch for FormField::List; key routing branches correctly on draft.focused (List-focused typing edits filter, List-focused Enter autofills without submitting, text-field Enter still submits when complete).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Behavior tests encoding intent (Karpathy Rule 8)</name>
  <files>src/tui/event.rs</files>
  <behavior>
    - Tab from Email focuses List; Tab from List focuses Name (3-way cycle in handle_key)
    - List focused + non-empty matched: Enter autofills new_name+new_email from highlighted author and STAYS on RenameForm (not Preview)
    - List focused: Char and Backspace update `filter`/`matched`, leaving new_name/new_email untouched
    - Single-author repo (excluded list empty): List focusable, Enter is a no-op, form still submittable via Name/Email
    - Source author is absent from the built items/matched
  </behavior>
  <action>
Add inline tests to the `mod tests` in src/tui/event.rs using the `make_rename_form_screen` helper from Task 1. Each test must state WHY in a comment (Rule 8). Add:
1. `test_rename_form_tab_cycles_through_list`: from a RenameForm, Tab (Name->Email), Tab (Email->List), Tab (List->Name) — assert focused at each step.
2. `test_rename_form_list_enter_autofills_and_stays`: build with source + others including ("Bob","bob@example.com"); set draft.focused=List; press Enter; assert still `Screen::RenameForm` and draft.new_name/new_email equal the highlighted (first matched) author. Must NOT be Preview.
3. `test_rename_form_list_typing_filters_not_text_fields`: focused=List; press Char then Backspace; assert `filter` reflects the edits and new_name/new_email remain empty.
4. `test_rename_form_empty_excluded_list_still_submittable`: build via the AuthorList->RenameForm transition on a repo with ONLY the source author (or pass others=&[]) so matched is empty; focus=List; Enter is a no-op (stays RenameForm, fields unchanged); then via Name/Email focus, fill both fields and assert is_complete()-gated Enter submits (or, in a bare repo, transitions to Preview). Keep the assertion realistic for the test repo used.
5. `test_rename_form_excludes_source_author`: assert the source identity does not appear in `items` nor `matched` of the built RenameForm.

Touch ONLY the test module in event.rs. No production code changes here. No Co-Authored-By trailers (Rule 10).
  </action>
  <verify>
    <automated>cd /Users/instinct/Desktop/working/git-author-reformer && rtk cargo test --lib tui</automated>
  </verify>
  <done>All five new behavior tests pass alongside the existing suite, encoding the locked decisions (3-way Tab, autofill-without-submit, filter-not-text routing, empty-list usability, source exclusion).</done>
</task>

</tasks>

<verification>
- `rtk cargo build` compiles clean.
- `rtk cargo test --lib tui` — all tests pass (existing + 5 new behavior tests).
- `rtk cargo clippy` — no new warnings introduced by these changes.
- Manual smoke (optional): run the binary in a repo with 2+ authors, choose Rename, select an author, Tab to the List, filter + Enter, confirm fields autofill and remain editable; verify a single-author repo still lets you rename via Name/Email.
</verification>

<success_criteria>
- The Rename form shows a third element: a filterable author list excluding the source author.
- Tab/BackTab cycles Name -> Email -> List -> Name.
- Selecting a list author autofills both fields; the fields remain editable.
- List-focused Enter autofills without submitting; text-field Enter still submits when complete.
- Empty excluded list keeps the form fully usable (no crash, no block).
- Only app.rs, event.rs, render.rs changed; no unrelated refactors; no AI co-author trailers in commits.
</success_criteria>

<output>
Create `.planning/quick/260524-kgx-add-author-selection-list-to-rename-form/260524-kgx-SUMMARY.md` when done.
</output>
