---
phase: 260524-kgx
reviewed: 2026-05-24T00:00:00Z
depth: quick
files_reviewed: 3
files_reviewed_list:
  - src/tui/app.rs
  - src/tui/event.rs
  - src/tui/render.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Task 260524-kgx: Code Review Report

## Findings & Disposition (orchestrator, 2026-05-24)

| Finding | Disposition | Rationale |
|---------|-------------|-----------|
| WR-01 BackTab cycles forward | **FIXED** (commit `f678917`) | Genuine regression in the 3-way ring. CONTEXT only said "Tab and BackTab cycles" without direction — the forward-alias was a planner scope-trim, not a user decision. Added `FormField::toggle_back()`; Shift+Tab now reverses. |
| WR-02 List-Enter "dead-end" when complete | **WON'T FIX (by design)** | This is locked CONTEXT decision D5: list-Enter autofills and *stays* on the list so the user can re-pick or edit; submission is via Tab→Name/Email then Enter. Encoded in `test_rename_form_list_enter_autofills_and_stays`. The reviewer worked without CONTEXT.md; advancing focus on autofill would violate the locked decision. |
| IN-01 source equality includes `commit_count` | **LEAVE** | `source` comes from the same `enumerate_authors` result as the list, so `commit_count` is consistent within one enumeration; the full-struct match is correct in practice. |
| IN-02 unused `items` field | **LEAVE** | Mirrors `Screen::AuthorList`, which also stores `items` it ignores. Removing only from `RenameForm` would diverge from the established pattern (CLAUDE.md Rule 3: match existing style). |

---

**Reviewed:** 2026-05-24
**Depth:** quick (pattern scan + targeted diff analysis)
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Reviewed the filterable author-selection list added to `Screen::RenameForm`. The 3-way Tab cycle, autofill-on-Enter, List-focused typing, empty-list safety, and source-author exclusion are structurally correct. One keyboard regression was introduced by this diff: `BackTab` is collapsed into the same handler as `Tab` and therefore cycles forward instead of backward through the new 3-way ring. A secondary issue is a UX dead-end when the List is focused and the form is complete: Enter silently does nothing instead of submitting. Two minor informational notes cover a fragile equality comparison and an unused struct field.

---

## Warnings

### WR-01: BackTab cycles forward instead of backward — keyboard regression

**File:** `src/tui/event.rs:220`
**Issue:** The diff collapsed `KeyCode::Tab | KeyCode::BackTab` into a single arm that always calls `toggle()` (Name→Email→List→Name). With the original 2-way toggle this was harmless. With the new 3-way ring, Shift+Tab now moves *forward* (same direction as Tab) instead of backward. A user on `Name` presses Shift+Tab expecting to land on `List`; they land on `Email`.

The prior code only handled `Tab`, and the pattern was already wrong for `BackTab` before this diff — but it was inert because the 2-way toggle produced a round-trip regardless of direction. The 3-way ring makes the defect observable for the first time.

**Fix:** Separate the two branches and add a reverse-direction helper:

```rust
// In FormField:
pub fn toggle_back(self) -> Self {
    match self {
        Self::Name  => Self::List,
        Self::Email => Self::Name,
        Self::List  => Self::Email,
    }
}

// In handle_key, replace the collapsed arm:
KeyCode::Tab => {
    let toggled = draft.focused.clone().toggle();
    draft.focused = toggled;
}
KeyCode::BackTab => {
    let toggled = draft.focused.clone().toggle_back();
    draft.focused = toggled;
}
```

---

### WR-02: List-focused Enter is a silent dead-end when form is complete

**File:** `src/tui/event.rs:224-231`
**Issue:** The `Enter if matches!(draft.focused, FormField::List)` arm fires first and autofills (or does nothing when `matched` is empty). The submission arm `Enter if draft.is_complete()` is never reached while List is focused. A user who tabs to the List, selects an author via Enter, has both fields filled — the form is now complete — and presses Enter again expecting submission. The second Enter is silently swallowed: matched has already been autofilled, so `matched.get(*selected)` succeeds again and overwrites the fields with the same values. No visual feedback, no transition to Preview.

The tests acknowledge this by always requiring the user to Tab away before submitting. This is an undocumented workflow gap that will surprise users: no footer hint says "Tab back to Name/Email to submit".

**Fix (minimal):** Either update the footer hint to make the required Tab explicit, or change the List-Enter arm to advance focus to Name after autofill so the next Enter submits:

```rust
KeyCode::Enter if matches!(draft.focused, FormField::List) => {
    if let Some(a) = matched.get(*selected) {
        draft.new_name  = a.name.clone();
        draft.new_email = a.email.clone();
    }
    // Advance focus so next Enter submits rather than re-autofilling.
    draft.focused = FormField::Name;
}
```

---

## Info

### IN-01: Source-exclusion equality is brittle against future `commit_count` changes

**File:** `src/tui/event.rs:183-184`
**Issue:** The exclusion filter `items.iter().filter(|a| **a != src)` uses full `AuthorIdentity` equality including `commit_count`. This works today because `src` is cloned directly from the same enumeration that produced `items`. If a future code path enumerates authors independently (e.g., from a different code path, producing a different `commit_count` for the same name+email), the source author would not be excluded and would appear in the list alongside itself.

**Fix:** Filter on name+email only:

```rust
let rest: Vec<_> = items
    .iter()
    .filter(|a| a.name != src.name || a.email != src.email)
    .cloned()
    .collect();
```

---

### IN-02: `items` field in `Screen::RenameForm` is populated but never read in production code

**File:** `src/tui/app.rs:27`, `src/tui/event.rs:211-217`, `src/tui/render.rs:19-26`
**Issue:** The `items: Vec<AuthorIdentity>` field is stored in `Screen::RenameForm` and matched with `..` in both `handle_key` and `render`. It is read only in the test at `event.rs:1707`. In production runtime, the field is dead weight after the `matched` list is derived from it. This mirrors the existing `Screen::AuthorList` pattern, where `items` is kept for re-filtering when the filter query is cleared — but in `Screen::RenameForm` the nucleo engine also holds the full dataset, so `items` is redundant.

**Fix (low priority):** Not a functional bug. If the nucleo engine is always re-queryable (it is, `apply_filter(nucleo, "")` recovers the full set), consider removing `items` from `Screen::RenameForm`. If `items` is intentionally kept for future features (e.g., re-seeding after refresh), add a comment explaining its purpose.

---

_Reviewed: 2026-05-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
