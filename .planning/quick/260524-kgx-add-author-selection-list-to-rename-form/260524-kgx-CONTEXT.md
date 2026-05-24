# Quick Task 260524-kgx: Add author selection list to rename form screen - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Task Boundary

On the "Rename an author" flow, the second screen (`Screen::RenameForm`) currently shows only two text fields: New name and New email. Add a third element to that screen: a selection list of git authors — like the first screen (`Screen::AuthorList`) — populated with all existing repo authors **except** the author currently being renamed (the `source`). Picking an author from the list autofills the New name and New email fields.

</domain>

<decisions>
## Implementation Decisions

### List action (what selecting does)
- Selecting an author from the list **autofills** the `New name` and `New email` fields with that author's `name` and `email`. The user can still edit both fields afterward before confirming. It is NOT an either/or mode — the list is a convenience that populates the existing draft fields.

### List behavior (filtering + focus)
- The list is **filterable like the first screen** (`Screen::AuthorList`): fuzzy match via nucleo as the user types.
- Focus model: `Tab` (and `BackTab`) cycles across **three** focus targets — Name field, Email field, and the author List.
- Typing routes to whichever target is focused: characters edit the Name/Email text when those are focused; characters update the list filter when the List is focused.
- When the List is focused, `Up`/`Down` move the list selection and `Enter` autofills the fields from the highlighted author. `Enter` while a text field is focused still submits the form (when complete), preserving today's behavior.

### List contents
- Populate from the same `enumerate_authors` result used by the first screen, **excluding** the `source` author (the one being renamed). Match on both name AND email (the full `AuthorIdentity`), since the source is a specific name+email identity.

### Display format
- Mirror `render_author_list` formatting exactly: `{commit_count:>4}  {name} <{email}>`, fuzzy filter row, "Authors (N match)" title. (Derived from "a list like the first screen" — not a separate decision.)

### Claude's Discretion
- Exact ratatui layout proportions for fitting header + name + email + filter row + list + footer into the rename screen.
- How focus is visually indicated for the list vs. the text fields (reuse existing bold/highlight conventions).

</decisions>

<specifics>
## Specific Ideas

- Existing reference implementations to mirror:
  - `render_author_list` in `src/tui/render.rs` (list + filter row rendering)
  - `Screen::AuthorList` arm in `src/tui/event.rs` (filter/navigation/Enter handling via `apply_filter` + nucleo)
  - `RenameDraft` / `FormField` in `src/tui/app.rs` (current two-field focus model — extend to three targets)

</specifics>

<canonical_refs>
## Canonical References

No external specs — requirements fully captured in decisions above.

</canonical_refs>

<constraints>
## Edge Cases / Constraints

- **Empty list:** If the repo has only the author being renamed, the excluded list is empty. The form must remain fully usable — fall back to name/email-only entry exactly as today. Do not block submission or crash on an empty list.

</constraints>
