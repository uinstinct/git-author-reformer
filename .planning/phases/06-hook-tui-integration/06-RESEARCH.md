# Phase 6: Hook TUI Integration — Research

**Researched:** 2026-05-21
**Domain:** Rust TUI state machine extension (ratatui + crossterm), hook engine API wiring
**Confidence:** HIGH — all findings derived directly from codebase inspection

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — discuss phase was skipped. All implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices. Use ROADMAP phase goal, success criteria, and codebase conventions
(TUI patterns in `src/tui/*` from Phase 3) to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HOOK-01 | TUI main menu shows "Add co-author auto-strip hook" as third option | MenuChoice extension pattern documented below |
| HOOK-02 | TUI main menu shows "Manage auto-strip hook" as fourth option; visible even when no hook installed | Same MenuChoice extension; Absent state → HookSuccess screen |
| HOOK-03 | "Add" shows current strip list then fuzzy-filterable co-author list reusing enumerate_coauthors | enumerate_coauthors reuse documented; single-screen layout recommended |
| HOOK-09 | "Manage" shows fuzzy-filterable strip email list; selecting removes entry | New StripList screen with Nucleo over Vec<String> |
| HOOK-11 | Both flows end on success screen showing final strip-list state from hook engine | New HookSuccess variant; HOOK-11 re-read mandate documented |
| HOOK-14 | Automated TUI/state-machine tests cover every user path | Existing test harness documented; make_test_app_with_stash helper needed |
</phase_requirements>

---

## Summary

Phase 6 wires two new main-menu flows (Add, Manage) to the Phase 5 hook engine. The existing
`App` + `Screen` + `handle_key` + `render` structure is straightforward to extend — all patterns
are established and the new screens follow the same bones as `CoAuthorList` and `Success`.

One critical structural finding: the SAFE-01/SAFE-02 preflight fires in `src/main.rs:20-21`,
**before** `ratatui::init()`. On a repo with stash entries the binary exits before the menu is
ever shown, making HOOK-12 and Success Criterion #5 impossible to satisfy without moving the
preflight calls. This is the single most consequential change this phase must make — it touches
existing shipped code and requires its own task with before/after test coverage.

**Primary recommendation:** Move preflight calls from `main.rs` into the `handle_key` MainMenu
Enter handler, gated to Rename (index 0) and Drop (index 1) only. Hook flows (Add, Manage) take
a different branch that skips preflight entirely.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Menu navigation (4 options) | TUI (event.rs) | app.rs (MenuChoice enum) | All keyboard dispatch lives in handle_key |
| Co-author enumeration (Add) | git/reader.rs | — | enumerate_coauthors already exists; reuse exactly |
| Strip list fuzzy selector (Manage) | TUI (event.rs + app.rs) | hook/mod.rs (read_strip_list) | New screen; hook engine is data source |
| Hook mutation | hook/mod.rs | — | install_strip / remove_strip are the only writers |
| Success state rendering | TUI (render.rs) | hook/mod.rs (read_strip_list) | Must re-read from hook engine after mutation |
| Preflight (Rename/Drop) | TUI (event.rs) | git/preflight.rs | Moved FROM main.rs — see §Critical Refactor below |

---

## Critical Refactor: Preflight Must Move from main.rs to event.rs

### Current location

`src/main.rs:19-21` [VERIFIED: read file]:
```rust
let repo = git::open_repo()?;
git::preflight::check_stash(&repo)?;    // line 20
git::preflight::check_worktrees(&repo)?; // line 21
```

These calls execute before `ratatui::init()` on line 31. If any check fails, the program
terminates with an error message before the TUI menu is rendered.

### Why this blocks HOOK-12 / Success Criterion #5

A repo with stash entries causes `check_stash` to return `Err(StashDetected)`. The binary
prints the error and exits. The user never sees the main menu. They cannot pick "Add" or
"Manage". There is no bypass path inside the TUI because the TUI never starts.

Success Criterion #5 explicitly requires both flows to reach their selectors on a repo with
stash entries. This is impossible with the current architecture.

### Required fix

**Remove** `check_stash` and `check_worktrees` from `src/main.rs:20-21`.

**Move them** into `src/tui/event.rs` inside the `Screen::MainMenu` Enter handler, **inside**
the branch for Rename (index 0) and Drop (index 1) only:

```rust
// event.rs — MainMenu Enter handler (after MenuChoice dispatch)
match MenuChoice::from_index(*selected) {
    MenuChoice::Rename => {
        // Preflight gates history-rewriting flows only
        if let Err(e) = crate::git::preflight::check_stash(&app.repo) {
            app.screen = Screen::Err(e.to_string());
            return;
        }
        if let Err(e) = crate::git::preflight::check_worktrees(&app.repo) {
            app.screen = Screen::Err(e.to_string());
            return;
        }
        // ... existing author-list load
    }
    MenuChoice::Drop => {
        // Same preflight gates
        // ... existing coauthor-list load
    }
    MenuChoice::AddHook => {
        // NO preflight — hook install does not rewrite history
        // ...
    }
    MenuChoice::ManageHook => {
        // NO preflight — hook manage does not rewrite history
        // ...
    }
}
```

`src/main.rs` retains only `git::open_repo()` — if you are not in a repo, that still fails
early, which is correct and desirable.

### Test impact on existing tests

All existing `event.rs` tests use `make_test_app()` (bare repo, no stash) or
`make_test_app_with_commits()` (no stash). None of the tests currently cover preflight.
Moving preflight into `event.rs` does NOT break existing tests — the stash check is a new
branch that only triggers when `refs/stash` exists, which none of the test repos create.

New tests needed: see §Validation Architecture.

---

## State Machine Extension

### Current Screen enum (`src/tui/app.rs:13-45`) [VERIFIED: read file]

```rust
pub enum Screen {
    MainMenu { selected: usize },
    AuthorList { items, filter, matched, nucleo, selected },
    RenameForm { source, draft },
    Preview { op, scan },
    CoAuthorList { items, filter, matched, nucleo, selected },
    Success { rewritten, remote_name, copied },
    Err(String),
}
```

### New variants needed

**`Screen::HookAddList`** — fuzzy co-author selector for the Add flow, PLUS a strip-list header
(to satisfy "show current strip list first" in HOOK-03). One screen, two visual areas.

```rust
Screen::HookAddList {
    // Strip list context shown in header
    current_strip: Vec<String>,          // from read_strip_list before entering
    // Co-author fuzzy selector (same machinery as CoAuthorList)
    items: Vec<CoAuthorEntry>,
    filter: String,
    matched: Vec<CoAuthorEntry>,
    nucleo: Nucleo<CoAuthorEntry>,
    selected: usize,
}
```

**`Screen::HookManageList`** — fuzzy selector over the strip list emails.

```rust
Screen::HookManageList {
    items: Vec<String>,       // strip emails from read_strip_list
    filter: String,
    matched: Vec<String>,     // filtered subset
    nucleo: Nucleo<String>,
    selected: usize,
}
```

**`Screen::HookSuccess`** — replaces (does not extend) the existing `Screen::Success` for hook
flows. Current `Screen::Success` is hard-wired to commit-rewrite output ("Rewrote N commits,
run git push --force-with-lease"). That is wrong for hook outcomes.

```rust
Screen::HookSuccess {
    state: crate::hook::HookState,   // live re-read from hook engine
}
```

`HookState::Absent` renders as "no entries configured" (empty-state view for Manage or
AlreadyStripped outcome). `HookState::Managed { emails }` renders the resulting list.
`HookState::NotToolManaged` should never reach this screen — treat it as `Screen::Err` earlier.

**`Screen::HookAlreadyStripped { email: String }`** — dedicated no-op screen for HOOK-05.
Renders "already stripped: <email>" with any-key-to-return behavior.

### Transition diagram

```
MainMenu(0=Rename)  --[Enter]--> [preflight] --> AuthorList
MainMenu(1=Drop)    --[Enter]--> [preflight] --> CoAuthorList
MainMenu(2=AddHook) --[Enter]--> [read_strip_list] --> HookAddList
MainMenu(3=Manage)  --[Enter]--> [read_strip_list] -->
    HookState::Absent           --> HookSuccess(Absent)     [empty state]
    HookState::Managed{emails}  --> HookManageList
    HookState::NotToolManaged   --> Err("foreign hook at <path>")

HookAddList         --[Enter on item]--> [install_strip] -->
    AddResult::Installed        --> [read_strip_list] --> HookSuccess(Managed)
    AddResult::AlreadyStripped  --> HookAlreadyStripped { email }
    Err(HookExists)             --> Err(msg)

HookManageList      --[Enter on item]--> [remove_strip] -->
    RemoveResult::Updated       --> [read_strip_list] --> HookSuccess(Managed)
    RemoveResult::HookDeleted   --> HookSuccess(Absent)
    RemoveResult::NotFound      --> Err(msg)   [defensive; shouldn't happen]
    Err(HookExists)             --> Err(msg)

HookSuccess         --[any key]--> should_exit = true
HookAlreadyStripped --[any key]--> MainMenu(selected: 2)  [return to menu — no-op outcome, program should not exit]
```

---

## MenuChoice Extension

### Current implementation (`src/tui/app.rs:96-119`) [VERIFIED: read file]

```rust
pub enum MenuChoice { Rename, Drop }

impl MenuChoice {
    pub fn from_index(i: usize) -> Self {
        if i == 0 { Self::Rename } else { Self::Drop }
    }
    pub fn all() -> [Self; 2] { [Self::Rename, Self::Drop] }
}
```

### Hardcoded count locations (ALL must be updated)

| File | Line | Current | Must become |
|------|------|---------|-------------|
| `src/tui/app.rs` | 117 | `[Self; 2]` / returns 2-element array | `[Self; 4]` / 4-element array |
| `src/tui/app.rs` | 101-108 | `if i == 0 ... else ...` | `match i { 0=>Rename, 1=>Drop, 2=>AddHook, _=>ManageHook }` |
| `src/tui/event.rs` | 40 | `% 2` (Down) | `% 4` |
| `src/tui/event.rs` | 41 | `% 2` (Up denominator) | `% 4` (and numerator `+2` → `+4`) |
| `src/tui/event.rs` | 43-79 | `if *selected == 0 { Rename } else { Drop }` | `match MenuChoice::from_index(*selected)` |
| `src/tui/render.rs` | 10-34 | 7-arm `match &app.screen` | add 4 new arms: HookAddList, HookManageList, HookSuccess, HookAlreadyStripped |

### Extended MenuChoice

```rust
pub enum MenuChoice { Rename, Drop, AddHook, ManageHook }

impl MenuChoice {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Self::Rename,
            1 => Self::Drop,
            2 => Self::AddHook,
            _ => Self::ManageHook,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rename    => "Rename an author",
            Self::Drop      => "Drop a co-author",
            Self::AddHook   => "Add co-author auto-strip hook",
            Self::ManageHook => "Manage auto-strip hook",
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::Rename, Self::Drop, Self::AddHook, Self::ManageHook]
    }
}
```

---

## Fuzzy Selector Reuse Strategy

### Existing machinery (`src/tui/app.rs:154-177`) [VERIFIED: read file]

`build_coauthor_nucleo(items: &[CoAuthorEntry]) -> Nucleo<CoAuthorEntry>` and
`apply_coauthor_filter(nucleo, query) -> Vec<CoAuthorEntry>` are already implemented and tested.

**The Add flow reuses them directly** — `HookAddList` carries a `Nucleo<CoAuthorEntry>` built
from `enumerate_coauthors(&app.repo)`. No new fuzzy function needed.

### Strip-list fuzzy selector (Manage flow)

The Manage flow filters over `Vec<String>` (emails), not `CoAuthorEntry`. A new pair of
functions is needed:

```rust
// To add to src/tui/app.rs
pub fn build_strip_nucleo(items: &[String]) -> Nucleo<String> {
    let nucleo = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = nucleo.injector();
    for item in items {
        let item = item.clone();
        injector.push(item.clone(), move |_, cols| {
            cols[0] = item.clone().into();
        });
    }
    nucleo
}

pub fn apply_strip_filter(nucleo: &mut Nucleo<String>, query: &str) -> Vec<String> {
    nucleo.pattern.reparse(0, query, CaseMatching::Ignore, Normalization::Smart, false);
    nucleo.tick(10);
    let snap = nucleo.snapshot();
    snap.matched_items(..).map(|m| m.data.clone()).collect()
}
```

These follow the exact pattern of `build_coauthor_nucleo` / `apply_coauthor_filter`.

### HOOK-11 re-read mandate

After `install_strip` or `remove_strip` succeeds, the `HookSuccess` screen MUST be populated
by a fresh `read_strip_list(&app.repo)` call — not by constructing a new state from the
`AddResult`/`RemoveResult` variant. The hook engine is the source of truth per HOOK-11.

Pattern:
```rust
// After install_strip returns Ok(AddResult::Installed { .. })
match crate::hook::read_strip_list(&app.repo) {
    Ok(state) => app.screen = Screen::HookSuccess { state },
    Err(e)    => app.screen = Screen::Err(e.to_string()),
}
```

---

## HookState Rendering Design

`HookState` variants (`src/hook/mod.rs:9-13`) [VERIFIED: read file]:

```rust
pub enum HookState {
    Absent,
    Managed { emails: Vec<String> },
    NotToolManaged(PathBuf),
}
```

(Note: CONTEXT.md uses `NoHook`/`Foreign` as aliases, but the code uses `Absent`/`NotToolManaged`.)

### Render logic for `Screen::HookSuccess { state }`

| State | Rendered content |
|-------|-----------------|
| `Absent` | "No hook installed — no emails configured." (empty state, also shown when last entry removed) |
| `Managed { emails }` | "Hook active — stripping N email(s): a@x.com, b@y.com" |
| `NotToolManaged` | Should never reach HookSuccess — caught earlier as Err |

For the remove-last-entry case: `remove_strip` returns `RemoveResult::HookDeleted`. The planner
must construct `HookSuccess { state: HookState::Absent }` directly (no re-read needed since we
know the hook was deleted). This is the one exception to the re-read mandate — the result is
unambiguous.

For all other cases: re-read per the HOOK-11 mandate.

### NotToolManaged at Manage entry

When the user picks Manage and `read_strip_list` returns `Ok(HookState::NotToolManaged(path))`,
transition to `Screen::Err` with the AppError::HookExists message. The user must remove or
rename the foreign hook file before using Manage. This is the cleanest and safest behavior
(analogous to HOOK-06's Add flow behavior).

---

## NLL Borrow Pattern (Required for event.rs additions)

The existing `Screen::Preview` arm in `src/tui/event.rs:158-196` [VERIFIED: read file] shows
the required pattern for screens where data must be extracted from the variant before the
screen is reassigned:

```rust
Screen::Preview { op, scan } => {
    // Step 1: Clone data out of the borrowed variant
    let op_clone = op.clone();
    let remote_name = scan.remote_name.clone();
    // Step 2: All reads complete; now match on key
    match key {
        KeyCode::Char('y') => {
            // Step 3: Use cloned data (app.screen borrow is now free)
            let result = match &op_clone { ... };
            // Step 4: Reassign app.screen
            app.screen = Screen::Success { ... };
        }
    }
}
```

New screen arms that call `app.repo` while destructuring `app.screen` must follow this pattern.
The standard approach: clone the email string (or whatever the selected item is) before the
`match key` block, then use the clone when calling `crate::hook::install_strip` or
`crate::hook::remove_strip`.

---

## Architecture Patterns

### Pattern 1: Add flow entry (MainMenu → HookAddList)

```rust
MenuChoice::AddHook => {
    // No preflight
    let strip_state = match crate::hook::read_strip_list(&app.repo) {
        Ok(s) => s,
        Err(e) => { app.screen = Screen::Err(e.to_string()); return; }
    };
    let current_strip = match strip_state {
        HookState::Managed { emails } => emails,
        HookState::Absent => vec![],
        HookState::NotToolManaged(_) => {
            // Could show a warning but still let user add (engine will refuse at write time)
            // Conservative: let them proceed; install_strip will return Err(HookExists) on Enter
            vec![]
        }
    };
    match crate::git::reader::enumerate_coauthors(&app.repo) {
        Ok(items) => {
            let mut nucleo = build_coauthor_nucleo(&items);
            let matched = apply_coauthor_filter(&mut nucleo, "");
            app.screen = Screen::HookAddList {
                current_strip,
                items,
                filter: String::new(),
                matched,
                nucleo,
                selected: 0,
            };
        }
        Err(e) => app.screen = Screen::Err(e.to_string()),
    }
}
```

### Pattern 2: HookAddList Enter (selection → install_strip → HookSuccess)

```rust
Screen::HookAddList { matched, selected, .. } => match key {
    KeyCode::Enter => {
        if let Some(target) = matched.get(*selected).cloned() {
            let email = target.email.clone();
            // Drop borrow before calling install_strip (NLL pattern)
            match crate::hook::install_strip(&app.repo, &email) {
                Ok(AddResult::Installed { .. }) => {
                    match crate::hook::read_strip_list(&app.repo) {
                        Ok(state) => app.screen = Screen::HookSuccess { state },
                        Err(e)    => app.screen = Screen::Err(e.to_string()),
                    }
                }
                Ok(AddResult::AlreadyStripped) => {
                    app.screen = Screen::HookAlreadyStripped { email };
                }
                Err(e) => app.screen = Screen::Err(e.to_string()),
            }
        }
    }
    // ... filter/nav keys mirror CoAuthorList pattern
}
```

### Pattern 3: Manage entry (MainMenu → HookManageList or HookSuccess[Absent] or Err)

```rust
MenuChoice::ManageHook => {
    match crate::hook::read_strip_list(&app.repo) {
        Ok(HookState::Absent) => {
            app.screen = Screen::HookSuccess { state: HookState::Absent };
        }
        Ok(HookState::Managed { emails }) => {
            let mut nucleo = build_strip_nucleo(&emails);
            let matched = apply_strip_filter(&mut nucleo, "");
            app.screen = Screen::HookManageList {
                items: emails,
                filter: String::new(),
                matched,
                nucleo,
                selected: 0,
            };
        }
        Ok(HookState::NotToolManaged(p)) => {
            app.screen = Screen::Err(
                format!("Foreign hook at {:?} — remove or rename it first.", p)
            );
        }
        Err(e) => app.screen = Screen::Err(e.to_string()),
    }
}
```

---

## Rendering Patterns

### HookAddList render

Three-zone layout (same structure as `render_coauthor_list`):
- Top: a `Paragraph` block titled "Current strip list" showing `current_strip` emails (or "no entries yet")
- Middle: filter input + fuzzy list of co-authors (same as `render_coauthor_list`)
- Bottom: hint line "type: filter  ↑/↓: move  Enter: select  Esc: back"

### HookManageList render

Same three-zone structure as `render_coauthor_list`, but items are plain email strings.
List title: "Strip list (N entries)" or "Strip list (empty)".

### HookSuccess render

Single `Paragraph` block:
- `HookState::Absent`: "No hook installed — no emails configured.\n\nAny key to exit."
- `HookState::Managed { emails }`: "Hook active — stripping N email(s):\n  a@x.com\n  b@y.com\n\nAny key to exit."

### HookAlreadyStripped render

Single `Paragraph` block:
"Already stripped: <email>\n\nThis email is already in the strip list.\n\nAny key to return."

Any-key returns to `Screen::MainMenu { selected: 2 }` (back to "Add" position).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Strip email fuzzy matching | Custom substring search | `build_strip_nucleo` / `apply_strip_filter` (Nucleo, same as existing) |
| Co-author enumeration in Add flow | Duplicate iterator | `enumerate_coauthors` from `src/git/reader.rs` |
| Hook state after mutation | Compute from AddResult/RemoveResult | `read_strip_list(&app.repo)` re-read (HOOK-11) |
| Terminal state management | Custom raw mode | ratatui::init() / ratatui::restore() — already wired in main.rs |
| Atomic hook write | Own tmp-file rename | `crate::hook::install_strip` / `crate::hook::remove_strip` — Phase 5 delivered these |

---

## Common Pitfalls

### Pitfall 1: Forgetting to update ALL hardcoded counts for the menu

**What goes wrong:** Adding new MenuChoice variants but leaving `% 2` in `event.rs:40-41` means
the new menu items are unreachable — the cursor wraps at index 1.
**Why it happens:** The modulus is a magic number in two lines of `event.rs`, not derived from
`MenuChoice::all().len()`.
**How to avoid:** Search for `% 2` in the file and replace both with `% 4`. Also update
`all()` return type from `[Self; 2]` to `[Self; 4]`, and replace the `if/else` dispatch with
a `match MenuChoice::from_index(*selected)`.
**Warning signs:** Menu navigation wraps after the second item in tests.

### Pitfall 2: Leaving preflight in main.rs and trying to "work around" it

**What goes wrong:** Any approach that tries to bypass preflight inside the TUI is doomed —
`check_stash` and `check_worktrees` fire before `ratatui::init()`. There is no TUI state when
they run.
**Why it happens:** The preflight was placed at startup in Phase 1/3 because at the time there
were only two operations, both of which required it.
**How to avoid:** Remove the two preflight calls from `src/main.rs:20-21` and move them into
`event.rs`'s MainMenu Enter handler, inside the Rename and Drop branches only.
**Warning signs:** A test that creates a repo with `refs/stash` and picks "Add" from the menu
never gets past `App::new()` (or rather never gets past `run()` in main.rs in real execution).

### Pitfall 3: Constructing HookSuccess state from AddResult/RemoveResult instead of re-reading

**What goes wrong:** `AddResult::Installed { count }` tells you how many entries are now in the
list, but the success screen must display the actual email strings. `count` is not sufficient;
you need `read_strip_list` to get the full list.
**Why it happens:** It looks like you can reconstruct the state locally (push email to cached
vec, etc.), and it is tempting to avoid the extra function call.
**How to avoid:** Always re-read via `read_strip_list(&app.repo)` after successful mutation.
One exception: `RemoveResult::HookDeleted` → construct `HookState::Absent` directly (the hook
is gone; reading it would return `Absent` anyway, but we can skip the round-trip).
**Warning signs:** Success screen shows cached email list that diverges from what is in the
file if the file was somehow modified externally between selection and display.

### Pitfall 4: Borrow-checker error when reading app.repo while app.screen is mutably borrowed

**What goes wrong:** `match &mut app.screen { Screen::HookAddList { matched, .. } => ... }`
creates a mutable borrow of `app`. Calling `app.repo` (or `crate::hook::install_strip(&app.repo, ...)`)
inside that match arm tries to borrow `app` again, which the borrow checker rejects.
**Why it happens:** The existing pattern is well-established in the file but easy to miss when
writing new arms.
**How to avoid:** Follow the NLL pattern from `Screen::Preview` arm (`event.rs:158-196`): clone
the needed data (`email.clone()`) before the `match key` block, then use the cloned values in
the key handler where `app.screen` is reassigned.
**Warning signs:** Compiler error "cannot borrow `app` as immutable because it is also borrowed
as mutable".

### Pitfall 5: HookState naming in CONTEXT.md does not match the code

**What goes wrong:** CONTEXT.md and some planning docs refer to `NoHook` and `Foreign`.
The actual enum in `src/hook/mod.rs:9-13` uses `Absent` and `NotToolManaged(PathBuf)`.
Using the wrong names anywhere (task specs, code) causes compile errors.
**Why it happens:** CONTEXT.md was written before Phase 5 finalized the naming.
**How to avoid:** Always use the code's names: `HookState::Absent`, `HookState::Managed { emails }`,
`HookState::NotToolManaged(PathBuf)`.

### Pitfall 6: Existing tests for main menu navigation use `% 2` semantics

**What goes wrong:** Tests `test_main_menu_down_increments_selected_mod_2` and
`test_main_menu_up_decrements_with_wrap` (`event.rs:306-333`) assert that Down from index 1
wraps to 0. After the menu grows to 4 items, Down from index 1 should go to index 2, not 0.
These test names encode the old assumption.
**Why it happens:** Tests were written when there were 2 menu items.
**How to avoid:** Update (not delete) these tests: change the wrap assertions to reflect 4-item
wrap, and rename the test functions to remove the "mod_2" suffix.
**Warning signs:** Tests pass with old modulus still in event.rs, giving false confidence.

---

## TUI Test Harness

### Existing infrastructure (`src/tui/event.rs:260-282`) [VERIFIED: read file]

All tests follow the same pattern:
1. `make_test_app()` — bare repo, no commits, no stash (uses `git2::Repository::init_bare`)
2. `make_test_app_with_commits()` — non-bare, one Alice commit
3. `make_test_app_with_coauthors()` — non-bare, one commit with `Co-authored-by: Bob` trailer
4. Drive via `handle_key(&mut app, KeyCode::...)` synchronously
5. Assert on `app.screen` variant

### New helper needed: `make_test_app_with_stash()`

Required for the HOOK-12 regression test (Success Criterion #6 — "Add/Manage on a repo with
stash entries does NOT hit preflight").

```rust
fn make_test_app_with_stash() -> (TempDir, App) {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    // Create initial commit
    let sig = git2::Signature::now("Alice", "alice@example.com").unwrap();
    let tree_oid = { let tb = repo.treebuilder(None).unwrap(); tb.write().unwrap() };
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    }
    // Create refs/stash by writing to the stash ref directly (no git binary available)
    // Find HEAD commit OID and create refs/stash pointing to it
    let head_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.reference("refs/stash", head_oid, false, "stash").unwrap();
    (dir, App::new(repo))
}
```

Note: This fakes `refs/stash` with an arbitrary OID. `check_stash` in `preflight.rs:1-6`
checks only `repo.find_reference("refs/stash").is_ok()` — the pointed-to OID does not matter.

### Test coverage required for HOOK-14

| Test name | What it verifies |
|-----------|-----------------|
| `test_main_menu_shows_four_options` | Render produces 4 items in MenuChoice::all() |
| `test_main_menu_routes_add_hook` | Enter on index 2 → HookAddList or HookSuccess(Absent) |
| `test_main_menu_routes_manage_hook` | Enter on index 3 → HookSuccess(Absent) or HookManageList |
| `test_add_hook_happy_path` | Select co-author → HookSuccess(Managed) |
| `test_add_hook_already_stripped` | Add duplicate → HookAlreadyStripped |
| `test_manage_empty_state` | Manage on no-hook repo → HookSuccess(Absent) |
| `test_manage_remove_single_entry` | Select email → HookSuccess(Managed with remaining) |
| `test_manage_remove_last_entry` | Select last email → HookSuccess(Absent) |
| `test_add_hook_no_preflight_with_stash` | Stash repo + Add → reaches HookAddList (not Err) |
| `test_manage_no_preflight_with_stash` | Stash repo + Manage → reaches HookSuccess or HookManageList (not Err) |
| `test_rename_still_hits_preflight_with_stash` | Stash repo + Rename → Screen::Err(StashDetected) |
| `test_drop_still_hits_preflight_with_stash` | Stash repo + Drop → Screen::Err(StashDetected) |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none (workspace-level) |
| Quick run command | `cargo test --lib tui::event` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command |
|--------|----------|-----------|-------------------|
| HOOK-01 | Main menu shows 4 options | unit | `cargo test --lib tui::event::tests::test_main_menu_shows_four_options` |
| HOOK-02 | Manage always visible; empty state works | unit | `cargo test --lib tui::event::tests::test_manage_empty_state` |
| HOOK-03 | Add shows strip list + co-author selector | unit | `cargo test --lib tui::event::tests::test_main_menu_routes_add_hook` |
| HOOK-09 | Manage shows fuzzy strip list | unit | `cargo test --lib tui::event::tests::test_main_menu_routes_manage_hook` |
| HOOK-11 | Success screen from hook engine re-read | unit | `cargo test --lib tui::event::tests::test_add_hook_happy_path` |
| HOOK-12 | No preflight on Add/Manage | unit | `cargo test --lib tui::event::tests::test_add_hook_no_preflight_with_stash` |
| HOOK-14 | All TUI paths covered | unit | `cargo test --lib tui::event` |

### Sampling Rate
- Per task commit: `cargo test --lib tui::event`
- Per wave merge: `cargo test`
- Phase gate: full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `make_test_app_with_stash()` helper function (add to `event.rs` test module)
- [ ] All tests in the HOOK-14 table above (new — none exist yet)
- [ ] Updated existing menu navigation tests to reflect 4-item wrap semantics

---

## Open Questions

1. **HookAddList — one screen or two?**
   - What we know: HOOK-03 says "shows currently-stripped emails, then a fuzzy-filterable
     co-author list". "Then" is ambiguous — one screen with two zones, or two separate screens?
   - What's unclear: A two-screen flow adds a new intermediate screen and more transitions.
   - Recommendation: One screen with a header panel showing current strip list and a body
     with the fuzzy co-author selector. Simpler, fewer transitions. If the current strip list
     is empty the header says "no entries yet". This is Claude's discretion.

2. **HookManageList on Err(HookExists) from remove_strip**
   - What we know: `remove_strip` can return `Err(HookExists)` if the hook was replaced by a
     foreign hook between when the user entered ManageList and when they pressed Enter.
   - What's unclear: This is a TOCTOU edge case with very low probability.
   - Recommendation: Transition to `Screen::Err` with the standard `HookExists` error message.
     Match the defensive pattern used in all other error paths.

---

## Environment Availability

Step 2.6: SKIPPED — no new external dependencies. ratatui, crossterm, nucleo, git2 are already
in Cargo.toml from Phase 3. The hook engine (src/hook/) was delivered in Phase 5.

---

## Security Domain

Hook TUI flows do not handle credentials, sessions, or user authentication. The hook engine
validates email characters for shell embedding safety (AppError::HookInvalidEmail) — that
validation is in Phase 5 and is not re-implemented here.

ASVS V5 (Input Validation): email validation for shell embedding is handled by
`crate::hook::render::validate_email_for_embedding` (called inside `install_strip`). The TUI
passes the selected email string verbatim; validation is the engine's responsibility.

---

## Sources

### Primary (HIGH confidence — codebase inspection)

All findings are from direct file reads of the current codebase at commit `b7671f2`:

- `src/main.rs` — preflight call sites (lines 20-21), run() structure
- `src/tui/app.rs` — Screen enum, MenuChoice, build_coauthor_nucleo, apply_coauthor_filter
- `src/tui/event.rs` — handle_key dispatch, NLL borrow pattern, existing test helpers
- `src/tui/render.rs` — rendering patterns for all existing screens
- `src/tui/mod.rs` — run_with_terminal loop
- `src/hook/mod.rs` — HookState, AddResult, RemoveResult, install_strip, remove_strip, read_strip_list
- `src/hook/parse.rs` — marker constants, detect_markers, extract_strip_list
- `src/git/reader.rs` — enumerate_coauthors (required reuse per HOOK-03)
- `src/git/preflight.rs` — check_stash, check_worktrees implementation
- `src/error.rs` — AppError variants
- `.planning/phases/05-hook-engine/05-04-SUMMARY.md` — Phase 5 delivery confirmation

---

## Metadata

**Confidence breakdown:**
- State machine extension: HIGH — derived directly from reading current Screen enum and event.rs
- Preflight refactor finding: HIGH — directly verified in main.rs; no bypass path exists
- Fuzzy reuse strategy: HIGH — build_coauthor_nucleo/apply_coauthor_filter verified in app.rs
- HookState naming: HIGH — verified against src/hook/mod.rs (differs from CONTEXT.md aliases)
- Test harness: HIGH — all existing helpers verified in event.rs test module

**Research date:** 2026-05-21
**Valid until:** 2026-06-21 (stable codebase; no external dependencies changing)
