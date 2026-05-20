# Phase 3: TUI + Integration - Research

**Researched:** 2026-05-20
**Domain:** ratatui TUI shell, nucleo fuzzy matching, git2 signature detection, Rust signal handling
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key constraints from ROADMAP (treated as locked):
- `ratatui::init()` and a SIGTERM handler (calling `ratatui::restore()`) must be the first code written — before any app logic — to prevent terminal stuck in raw mode on panic or signal
- Target author entry is a free-text two-field form (new name + new email), not a second list picker
- Tech stack: ratatui 0.30.0, crossterm 0.29.0, nucleo 0.5.0 — decided; no alternatives considered

### Claude's Discretion
All implementation choices not listed above are at Claude's discretion.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

## Summary

Phase 3 wires a full ratatui TUI around the git layer built in Phases 1-2. The architecture is clear: a central `App` struct holding a `Screen` enum drives both renders and input dispatch. All existing rewrite functions are present and correct — but there is one critical missing API that this phase must add before the TUI layer can proceed: a **read-only scan/preview** function that walks commits and returns affected count, signed-commit count, annotated-tag warnings, and notes-ref presence. Without it, RENAME-05 and DROP-04 (show affected count before confirming) cannot be implemented without redundantly re-walking inside the TUI.

The scan function must perform the same topological walk as the rewrite functions — including cascade tracking — to produce an exact affected count. A simple count of identity-matching commits produces a lower (wrong) number because the rewrite also touches all descendants of rewritten commits.

The standard ratatui 0.30 event loop uses `terminal.draw()` + `crossterm::event::read()` in a tight loop. `ratatui::init()` handles raw mode, alternate screen, and panic hook installation automatically. SIGTERM requires an explicit handler (`signal-hook` 0.4 crate) because panic hooks do not fire on OS signals. nucleo's high-level `Nucleo<T>` API is the correct choice — the low-level `nucleo-matcher` crate requires manual buffer management that is not worth the effort for a static author list.

**Primary recommendation:** Implement `git::scan` module first (read-only topological revwalk returning `RewritePreview`), then build the TUI state machine bottom-up: state types → key handler → renderer → wiring to main.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Terminal raw mode + alternate screen | TUI layer (main.rs) | — | `ratatui::init()` owns this; must be first call |
| SIGTERM/panic cleanup | TUI layer (main.rs) | — | Must be registered before any app logic |
| App state machine | TUI layer (src/tui/) | — | Pure Rust types; no git2 dependency |
| Fuzzy author/co-author filtering | TUI layer | — | nucleo runs in TUI event loop; git data is static once loaded |
| Read-only commit scan (count + warnings) | git layer (src/git/scan.rs) | — | Shared revwalk logic with cascade tracking; TUI calls result, never owns walk |
| Rewrite execution | git layer (src/git/rewrite.rs) | — | Already implemented in Phase 2 |
| Remote name detection | git layer (src/git/scan.rs) | — | `repo.remotes()` → prefer "origin", else first, else None |
| Signature detection (GPG/SSH) | git layer (src/git/scan.rs) | — | `commit.header_field_bytes("gpgsig")` / `"sshsig"` on cascaded set |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ratatui` | `0.30.0` | TUI rendering, widgets, layout | Project-mandated; current stable release [VERIFIED: cargo search] |
| `crossterm` | `0.29.0` | Terminal backend (raw mode, events, cursor) | ratatui's default backend; cross-platform [VERIFIED: cargo search] |
| `nucleo` | `0.5.0` | Fuzzy-filterable author/co-author lists | Same engine as Helix editor; high-level `Nucleo<T>` API with threadpool [VERIFIED: cargo search] |
| `signal-hook` | `0.4` | SIGTERM handler to call `ratatui::restore()` | 151M downloads; maintained by vorner; `flag::register` API unchanged in 0.4 [VERIFIED: crates.io + docs.rs] |

### Not Needed
| Library | Reason |
|---------|--------|
| `tui-textarea` | Single-line fields only; a custom two-field widget is ~40 lines and avoids a heavy dependency [ASSUMED] |
| `tokio` / `async-std` | TUI event loops are synchronous; crossterm event polling is blocking with timeout |

**Installation (additions to Cargo.toml):**
```toml
ratatui = "0.30"
crossterm = "0.29"
nucleo = "0.5"
signal-hook = "0.4"
```

**Version verification:**
```
ratatui:      cargo search -> 0.30.0  [VERIFIED: cargo search]
crossterm:    cargo search -> 0.29.0  [VERIFIED: cargo search]
nucleo:       cargo search -> 0.5.0   [VERIFIED: cargo search]
signal-hook:  cargo search -> 0.4.4 (latest stable) [VERIFIED: cargo search]
              flag::register(signal, Arc<AtomicBool>) API confirmed unchanged in 0.4
tui-textarea: cargo search -> 0.7.0 (not needed)
```

## Package Legitimacy Audit

slopcheck was not available at research time. All packages below are tagged `[ASSUMED]`. The planner must gate each install behind a `checkpoint:human-verify` task.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `ratatui` | crates.io | ~3 yrs | Very high | github.com/ratatui/ratatui | [ASSUMED] | Approved — project-mandated; well-known |
| `crossterm` | crates.io | ~6 yrs | Very high | github.com/crossterm-rs/crossterm | [ASSUMED] | Approved — ratatui's standard backend |
| `nucleo` | crates.io | ~2 yrs | High | github.com/helix-editor/helix (extracted) | [ASSUMED] | Approved — project-mandated |
| `signal-hook` | crates.io | ~6 yrs | 151M total | github.com/vorner/signal-hook | [ASSUMED] | Approved — widely used, maintained |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable at research time — all packages tagged `[ASSUMED]`.*

## Architecture Patterns

### System Architecture Diagram

```
User keyboard input
        |
        v
crossterm::event::read()
        |
        v
+------ App State Machine -------+
|  Screen enum:                   |
|    MainMenu                     |
|    AuthorList(filter, nucleo)   |
|    CoAuthorList(filter, nucleo) |
|    RenameForm(fields, cursor)   |
|    Preview(RewritePreview)      |
|    Executing                    |
|    Success(outcome)             |
|    Err(message)                 |
+------ handle_key(state, key) --+
        |
        v
git layer calls (read-only until Preview → Executing)
  git::scan::scan_rename(repo, name, email) -> RewritePreview
  git::scan::scan_drop(repo, email) -> RewritePreview
  git::rewrite::rewrite_author(...)
  git::rewrite::drop_coauthor(...)
        |
        v
terminal.draw(|f| render(f, &state))
  renders to CrosstermBackend (alternate screen)
```

### New Module Required: `src/git/scan.rs`

This is the load-bearing gap. The existing `rewrite_author` and `drop_coauthor` functions execute and return a count. But RENAME-05 and DROP-04 require showing the affected count **before** user confirmation, and SAFE-03/04/05 require showing non-blocking warnings. A read-only scan function must be added.

**Critical: cascade tracking is required for exact count.** The rewrite functions count every commit that is either identity-matching OR has a remapped parent (`any_parent_remapped`). The scan must replicate this logic — it cannot just count identity-matching commits. A naive count will be lower than the actual rewrite count and will lie to the user.

```rust
// Source: design derived from rewrite.rs cascade logic + requirements RENAME-05, DROP-04, SAFE-03, SAFE-04, SAFE-05, OUT-01
pub struct RewritePreview {
    pub affected_count: usize,
    pub signed_commit_count: usize,        // SAFE-03: GPG/SSH signatures in cascaded set
    pub annotated_tags_affected: Vec<String>, // SAFE-04: tag names pointing at cascaded set
    pub has_notes_ref: bool,               // SAFE-05: refs/notes/commits exists
    pub remote_name: Option<String>,       // OUT-01: "origin" preferred, else first remote, else None
}

pub fn scan_rename(
    repo: &git2::Repository,
    old_name: &str,
    old_email: &str,
) -> Result<RewritePreview, crate::error::AppError>

pub fn scan_drop(
    repo: &git2::Repository,
    target_email: &str,
) -> Result<RewritePreview, crate::error::AppError>
```

**Scan algorithm (same topological walk as rewrite, without writing):**

```rust
// Source: modelled after rewrite_author in src/git/rewrite.rs
fn scan_rename_inner(repo, old_name, old_email) -> Result<RewritePreview> {
    let mut revwalk = /* push_glob refs/heads/*, refs/tags/*, TOPOLOGICAL|REVERSE */;
    let mut would_remap: HashSet<Oid> = HashSet::new();  // tracks cascade set

    for oid in revwalk {
        let commit = repo.find_commit(oid)?;
        let identity_matches = commit.author().name() == old_name && ...;
        let any_parent_remapped = commit.parent_ids().any(|p| would_remap.contains(&p));
        if identity_matches || any_parent_remapped {
            would_remap.insert(oid);
        }
    }

    // Sign check: iterate would_remap, check header_field_bytes("gpgsig"/"sshsig")
    // Tag check: walk refs/tags/*, check if tag target_id is in would_remap
    // Notes check: find_reference("refs/notes/commits")
    // Remote: repo.remotes() -> prefer "origin"
}
```

SAFE-03 signed count and SAFE-04 annotated tag names must be computed from the `would_remap` set, not from the full commit history.

### Recommended Project Structure
```
src/
├── git/
│   ├── mod.rs          # open_repo(), pub use scan
│   ├── types.rs        # AuthorIdentity, CoAuthorEntry (existing)
│   ├── preflight.rs    # check_stash, check_worktrees (existing)
│   ├── reader.rs       # enumerate_authors, enumerate_coauthors (existing)
│   ├── rewrite.rs      # rewrite_author, drop_coauthor (existing)
│   └── scan.rs         # NEW: RewritePreview, scan_rename, scan_drop
├── tui/
│   ├── mod.rs          # pub use app::App; pub fn run(repo) -> Result<>
│   ├── app.rs          # App struct, Screen enum, AppState
│   ├── event.rs        # handle_key(app, key) -> ()
│   └── render.rs       # render(frame, app) — dispatches to per-screen fns
├── error.rs            # AppError (existing, may need TuiError variant)
├── lib.rs              # pub mod git; pub mod tui
└── main.rs             # SIGTERM handler, ratatui::init(), tui::run()
```

### Pattern 1: ratatui Init + SIGTERM + Panic Hook

`ratatui::init()` in 0.30 installs a panic hook that calls `ratatui::restore()` before the panic message is printed. However, **SIGTERM does not trigger panic hooks** — it terminates the process directly. An explicit SIGTERM handler is required to avoid leaving the terminal in raw mode.

```rust
// Source: [CITED: docs.rs/ratatui 0.30 init] [CITED: docs.rs/signal-hook 0.4]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. SIGTERM flag — must be registered BEFORE ratatui::init()
    let term_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(
        signal_hook::consts::SIGTERM,
        Arc::clone(&term_flag),
    )?;

    // 2. ratatui::init() — installs panic hook + raw mode + alternate screen
    let mut terminal = ratatui::init();

    // 3. Run app; check term_flag in the event loop
    let result = run_app(&mut terminal, term_flag);

    // 4. Always restore — even on error path
    ratatui::restore();

    // 5. Propagate error after restore (so terminal is clean before printing)
    result?;
    Ok(())
}
```

The event loop checks `term_flag` at the top of each iteration:
```rust
fn run_app(terminal: &mut DefaultTerminal, term_flag: Arc<AtomicBool>) -> io::Result<()> {
    let mut app = App::new(/* ... */);
    loop {
        if term_flag.load(Ordering::Relaxed) { break; }
        terminal.draw(|f| render(f, &app))?;
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key.code);
                }
            }
        }
        if app.should_exit() { break; }
    }
    Ok(())
}
```

**Critical:** `KeyEventKind::Press` filter prevents double-fire on Windows (key-repeat and key-release also emit events). [CITED: ratatui.rs counter-app tutorial]

### Pattern 2: App State Machine

The multi-screen flow maps cleanly to a Rust enum. State transitions are pure functions — no side effects — making them fully testable:

```rust
// Source: [ASSUMED] — standard ratatui app pattern
pub enum Screen {
    MainMenu { selected: usize },
    AuthorList {
        items: Vec<AuthorIdentity>,
        filter: String,
        nucleo: Nucleo<AuthorIdentity>,
        selected: usize,
    },
    CoAuthorList {
        items: Vec<CoAuthorEntry>,
        filter: String,
        nucleo: Nucleo<CoAuthorEntry>,
        selected: usize,
    },
    RenameForm {
        source: AuthorIdentity,
        new_name: String,
        new_email: String,
        focused_field: FormField,  // Name | Email
    },
    Preview {
        operation: PendingOp,
        scan: RewritePreview,
    },
    Executing,
    Success {
        rewritten: usize,
        remote_name: Option<String>,
    },
    Err(String),
}

pub enum PendingOp {
    Rename { source: AuthorIdentity, new_name: String, new_email: String },
    Drop { target: CoAuthorEntry },
}
```

State transitions happen in `event.rs`: `handle_key(&mut app, key)` mutates `app.screen` in-place via `app.screen = Screen::NextState { ... }`.

### Pattern 3: nucleo Integration

nucleo's `Nucleo<T>` API is threaded and designed for interactive use. For an author list (typically < 200 entries), `Nucleo::new()` with a no-op callback and `tick(10)` completes instantly:

```rust
// Source: [CITED: docs.rs/nucleo 0.5]
use nucleo::{Config, Nucleo};
use nucleo::pattern::{CaseMatching, Normalization};

fn build_nucleo(items: Vec<AuthorIdentity>) -> Nucleo<AuthorIdentity> {
    let mut nucleo = Nucleo::new(
        Config::DEFAULT,
        Arc::new(|| {}),  // no-op notify callback
        None,             // use default thread count
        1,                // one column (the display string)
    );
    let injector = nucleo.injector();
    for item in &items {
        let display = format!("{} <{}>", item.name, item.email);
        injector.push(item.clone(), move |_, cols| {
            cols[0] = display.clone().into();
        });
    }
    nucleo
}

fn apply_filter(nucleo: &mut Nucleo<AuthorIdentity>, query: &str) -> Vec<AuthorIdentity> {
    nucleo.pattern.reparse(0, query, CaseMatching::Ignore, Normalization::Smart, false);
    nucleo.tick(10);  // wait up to 10ms — instant for small lists; avoids stale results
    let snap = nucleo.snapshot();
    snap.matched_items(..).map(|m| m.data.clone()).collect()
}
```

Each time the user types a character, `apply_filter` is called and the list re-renders with the filtered results.

### Pattern 4: Signature Detection (SAFE-03)

GPG-signed commits embed a `gpgsig` header; SSH-signed commits use `sshsig`. Both are accessible via `commit.header_field_bytes(field)`:

```rust
// Source: [CITED: docs.rs/git2 0.21 Commit::header_field_bytes]
fn commit_is_signed(commit: &git2::Commit) -> bool {
    commit.header_field_bytes("gpgsig").is_ok()
        || commit.header_field_bytes("sshsig").is_ok()
}
```

`header_field_bytes` returns `Err` when the field is absent, `Ok(Buf)` when present. Treat `Ok(_)` as "signature present" regardless of buffer content.

**Important:** The signed count in `RewritePreview` must be computed from the cascaded `would_remap` set only — not all commits in the repo. A signed commit that is not in the rewrite set needs no warning.

### Pattern 5: Notes-Ref Detection (SAFE-05)

```rust
// Source: [CITED: docs.rs/git2 0.21 Repository::find_reference, note_default_ref]
fn has_notes_ref(repo: &git2::Repository) -> bool {
    // Check configured default first; fall back to canonical location
    let default_ref = repo.note_default_ref().unwrap_or_else(|_| "refs/notes/commits".to_string());
    repo.find_reference(&default_ref).is_ok()
        || repo.find_reference("refs/notes/commits").is_ok()
}
```

`repo.note_default_ref()` returns the configured notes ref (may differ from the standard via `notes.ref` config). Checking both the configured default and the canonical `refs/notes/commits` catches the common case. [ASSUMED — non-standard custom notes refs beyond these two are out of scope for v1]

### Pattern 6: Remote Name Detection (OUT-01)

```rust
// Source: [CITED: docs.rs/git2 0.21 Repository::remotes()]
fn detect_remote_name(repo: &git2::Repository) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    let names: Vec<&str> = remotes.iter().flatten().collect();
    if names.contains(&"origin") {
        Some("origin".to_string())
    } else {
        names.first().map(|s| s.to_string())
    }
}
```

If no remotes are configured, `remote_name` is `None` and the success screen shows a generic `<remote>` placeholder.

### Pattern 7: Two-Field Form Without tui-textarea

For single-line name and email entry, a custom approach is more appropriate than `tui-textarea` (which is optimised for multi-line editors). The form state tracks:

```rust
struct RenameFormState {
    new_name: String,
    new_email: String,
    focused_field: FormField,  // Name | Email
}
```

Key handling: printable characters append to the focused field's `String`; Backspace pops the last char (`String::pop()`); Tab/Shift-Tab switches focus; Enter submits if both fields are non-empty. Rendered as two `Paragraph` widgets inside `Block` containers, with the focused field styled differently (e.g., bold border).

**Cursor visibility:** Call `frame.set_cursor_position((x + field_content.len() as u16, y))` for the focused field so the terminal cursor appears at the right position inside the input area. Without this the user cannot tell where typing will land.

### Anti-Patterns to Avoid

- **Running scan inside the TUI render function:** `scan_rename`/`scan_drop` do a full revwalk. Call them exactly once during the transition to `Screen::Preview`. Store `RewritePreview` in the screen variant; the render function reads from it.
- **Simple count (not cascade-tracking) in scan:** Must track `would_remap` set (same logic as rewrite) to produce the exact count RENAME-05 requires. A naive identity-match count will differ from what `rewrite_author` reports.
- **Using `commit.message()` instead of `commit.message_raw()`:** `message()` strips leading newlines. The existing rewrite code correctly uses `message_raw()` — scan code must too for consistency.
- **Calling `ratatui::restore()` only on the happy path:** Always call it: use a `defer`-style pattern or explicit cleanup before every return.
- **Matching all `KeyEvent` kinds:** Filter to `KeyEventKind::Press` only; otherwise release events double-fire on Windows.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzy text matching | Custom edit-distance filter | `nucleo::Nucleo<T>` | nucleo outperforms fzf; handles CJK, case-folding, Unicode normalisation |
| Terminal raw mode setup | Manual `termios` manipulation | `ratatui::init()` | Handles raw mode, alternate screen, panic hook in one call |
| Signal handling | `unsafe` `libc::signal()` | `signal-hook` flag API | Thread-safe, safe Rust, registered in 3 lines |
| Layout calculation | Manual row/column arithmetic | `Layout::vertical/horizontal` with `Constraint` | Cassowary solver handles resize automatically |
| Bordered panels | Manual border drawing | `Block::bordered().title()` | Widget system handles Unicode borders, title truncation |

**Key insight:** ratatui's widget system handles all terminal geometry. The application code should only deal with state and transitions.

## TDD Strategy for TUI Code

TDD mode is enabled. The TUI layer has three distinct test surfaces with different strategies:

### Fully TDD-able (write tests first, then implement)

1. **`git::scan` module** — pure revwalk logic returning a struct. Tests use `tempfile` + `git2` fixture repos (same pattern as Phase 1-2 tests). Write tests for: exact affected count with cascade, signed commit detection, annotated tag detection, notes-ref detection, remote name detection.

2. **App state transitions** — `handle_key(app, key)` is a pure mutation on a Rust struct. Tests check: `MainMenu + Enter → AuthorList`, `AuthorList + Esc → MainMenu`, `RenameForm + Tab → focus switches`, `Preview + 'y' → Executing`, `Preview + 'n'/'q' → MainMenu`.

3. **Form validation** — rejection of empty new name or email before allowing submission.

### Snapshot-testable (write test, verify rendered output)

4. **Critical screens with `TestBackend`** — `ratatui::backend::TestBackend` renders to a memory buffer and exposes `assert_buffer_lines()`. Use for: confirmation screen showing correct affected count, success screen showing force-push command. [CITED: docs.rs/ratatui TestBackend]

### Manual-only (document why, not TDD)

5. **Full end-to-end keyboard flow** — requires interactive terminal; not automatable without a pty harness like `expect`. Document as manual smoke test.

6. **SIGTERM behaviour** — cannot send OS signals in standard unit tests. Document as manual check: `kill -TERM <pid>` while running, verify terminal restored.

## Common Pitfalls

### Pitfall 1: Terminal Not Restored After Panic or Signal

**What goes wrong:** If the application panics or receives SIGTERM before `ratatui::restore()` is called, the terminal stays in raw mode / alternate screen. The user's shell becomes unusable.

**Why it happens:** `ratatui::init()` installs a panic hook, but panic hooks do not fire on `SIGTERM`. The process is simply killed.

**How to avoid:** Register the SIGTERM flag before `ratatui::init()`. Check the flag at the top of the event loop. Always call `ratatui::restore()` in all exit paths. [CITED: ratatui init docs, signal-hook docs]

**Warning signs:** Shell prompt appears corrupted after killing the process; cursor invisible; characters not echoed.

### Pitfall 2: Scan Returns Wrong Count Due to Missing Cascade Logic

**What goes wrong:** Scan counts only identity-matching commits. The actual rewrite reports a higher count (cascade descendants). The confirmation prompt says "17 commits affected" but `rewrite_author` touches 23. RENAME-05 says exact count.

**Why it happens:** The simpler scan algorithm misses `any_parent_remapped` from the rewrite logic.

**How to avoid:** Scan must maintain a `would_remap: HashSet<Oid>` and insert any commit where `identity_matches || any_parent_remapped`, mirroring the rewrite walk exactly.

### Pitfall 3: Scan Function Called in Render Loop

**What goes wrong:** If `scan_rename` or `scan_drop` is called inside `render()` (which runs every frame), the revwalk executes hundreds of times per second.

**Why it happens:** Render functions look like a natural place to "compute what to show."

**How to avoid:** Call scan functions exactly once, during the state transition that creates `Screen::Preview`. Store the `RewritePreview` in the screen variant.

### Pitfall 4: Double Key Events on Windows

**What goes wrong:** Every keypress triggers two events (press + release), causing double navigation or double-character input.

**Why it happens:** crossterm emits `KeyEventKind::Press`, `KeyEventKind::Repeat`, and `KeyEventKind::Release` on Windows.

**How to avoid:** Filter `if key.kind == KeyEventKind::Press` in `handle_key`. [CITED: ratatui counter-app tutorial]

### Pitfall 5: nucleo `tick(0)` on First Query Returns Stale Results

**What goes wrong:** After calling `nucleo.pattern.reparse(...)`, the background thread may not have finished computing matches when `tick(0)` returns immediately.

**Why it happens:** `Nucleo<T>` runs matching on a threadpool. `tick(timeout_ms)` waits at most `timeout_ms` milliseconds.

**How to avoid:** Use `tick(10)` (10ms) for a list of < 200 items. Imperceptible to the user; guarantees match pass completes. [CITED: docs.rs/nucleo 0.5]

### Pitfall 6: Empty Remote List Panic

**What goes wrong:** Unwrapping `repo.remotes()` or `names.first()` panics when no remotes exist.

**Why it happens:** A local-only repo has no remotes.

**How to avoid:** `detect_remote_name` returns `Option<String>`. Show `<remote>` placeholder in success screen when `None`.

## Code Examples

### ratatui main loop (minimal)

```rust
// Source: [CITED: docs.rs/ratatui 0.30 init] [CITED: ratatui.rs counter-app tutorial]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

fn run_app(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key.code);
                }
            }
        }
        if app.should_exit() {
            break;
        }
    }
    Ok(())
}
```

### List widget with ListState

```rust
// Source: [CITED: docs.rs/ratatui 0.30 List widget]
use ratatui::widgets::{Block, List, ListItem, ListState};
use ratatui::style::{Style, Modifier};

fn render_author_list(frame: &mut Frame, area: Rect, items: &[String], state: &mut ListState) {
    let list_items: Vec<ListItem> = items.iter().map(|s| ListItem::new(s.as_str())).collect();
    let list = List::new(list_items)
        .block(Block::bordered().title("Select Author"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, state);
}
```

### Layout split

```rust
// Source: [CITED: docs.rs/ratatui 0.30 Layout]
use ratatui::layout::{Constraint, Layout};

let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Fill(1),
    Constraint::Length(2),
]).areas(frame.area());
```

### GPG/SSH signature check

```rust
// Source: [CITED: docs.rs/git2 0.21 Commit::header_field_bytes]
fn commit_is_signed(commit: &git2::Commit) -> bool {
    commit.header_field_bytes("gpgsig").is_ok()
        || commit.header_field_bytes("sshsig").is_ok()
}
```

### Cursor positioning in text input

```rust
// Source: [CITED: docs.rs/ratatui 0.30 Frame::set_cursor_position]
// After rendering the input paragraph inside `input_area`:
let cursor_x = input_area.x + 1 + field_text.len() as u16; // +1 for block border
let cursor_y = input_area.y + 1;
frame.set_cursor_position((cursor_x, cursor_y));
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tui` crate (archived) | `ratatui` (fork) | 2023 | ratatui is the maintained successor; same API surface, actively developed |
| `actions-rs/toolchain` | `dtolnay/rust-toolchain` | 2023 | actions-rs archived; already documented in CLAUDE.md |
| crossterm re-exported from ratatui | Direct `crossterm` dependency | ratatui 0.26+ | ratatui no longer re-exports crossterm event types; add `crossterm` directly to Cargo.toml |

**Deprecated/outdated:**
- `tui-rs`: Archived. ratatui is its active fork. Do not use.
- `actions-rs/*`: Archived. Already in CLAUDE.md "What NOT to Use".

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tui-textarea` is overkill for single-line fields; custom widget is preferred | Standard Stack | If custom widget proves tricky (cursor rendering), tui-textarea is a fallback; adds one dependency |
| A2 | Checking `refs/notes/commits` plus `note_default_ref()` covers all practical notes setups | Pattern 5 | If user stores notes under a fully custom ref not matching these, warning is missed; acceptable for v1 |
| A3 | `tick(10)` is sufficient to guarantee nucleo match completion for < 200 authors | Pattern 3 | If a slow machine takes > 10ms, filtered list lags one keystroke; increase timeout |

## Open Questions (RESOLVED)

1. **notes check scope (SAFE-05)**
   - What we know: `repo.note_default_ref()` returns the configured default; `refs/notes/commits` is the canonical location
   - What's unclear: Whether edge-case repos with custom notes refs (neither "origin" nor default) need coverage
   - RESOLVED: Check both `note_default_ref()` and `refs/notes/commits` — covers all standard cases; custom notes refs outside these are out of scope for v1

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | Yes | 1.92.0 | — |
| cargo | Build | Yes | 1.92.0 | — |
| ratatui 0.30 | TUI rendering | Not yet in Cargo.toml | — | Add to Cargo.toml |
| crossterm 0.29 | Terminal backend | Not yet in Cargo.toml | — | Add to Cargo.toml |
| nucleo 0.5 | Fuzzy filtering | Not yet in Cargo.toml | — | Add to Cargo.toml |
| signal-hook 0.4 | SIGTERM handling | Not yet in Cargo.toml | — | Add to Cargo.toml |

**Missing dependencies with no fallback:** None (all are addable to Cargo.toml; no OS-level tooling gaps).

## Project Constraints (from CLAUDE.md)

| Directive | Impact on This Phase |
|-----------|---------------------|
| Think before coding — state assumptions, push back | Phase plan must state all state machine design choices explicitly |
| Simplicity first — no features beyond what was asked | No animation, no mouse support, no colour themes beyond bold/reverse |
| Surgical changes — touch only what you must | `main.rs` is replaced by TUI entry; `src/git/` gets only `scan.rs` added |
| No external tools — binary works without git installed | All git operations via git2; already enforced in phases 1-2 |
| Static linking required | No change needed — already in Cargo.toml profile.release |
| ratatui::init() + SIGTERM handler must be first code written | Wave 0 plan must be: add Cargo deps + write main.rs init/restore shell first |
| Target author entry is two-field form, not a second list picker | `Screen::RenameForm` has `new_name` + `new_email` fields; no `AuthorList` for target |
| Tech stack: ratatui 0.30, crossterm 0.29, nucleo 0.5 — decided | No alternatives researched |

## Sources

### Primary (HIGH confidence)
- [docs.rs/ratatui/0.30.0](https://docs.rs/ratatui/0.30.0/ratatui/index.html) — init/restore behaviour, TestBackend, List widget, Paragraph, Block, Layout
- [docs.rs/git2/0.21.0/git2/struct.Commit](https://docs.rs/git2/0.21.0/git2/struct.Commit.html) — header_field_bytes, raw_header, raw_header_bytes
- [docs.rs/git2/0.21.0/git2/struct.Repository](https://docs.rs/git2/0.21.0/git2/struct.Repository.html) — remotes(), notes(), note_default_ref(), references_glob()
- [docs.rs/nucleo/0.5.0](https://docs.rs/nucleo/0.5.0/nucleo/struct.Nucleo.html) — Nucleo<T> API, Injector, Snapshot, tick()
- [docs.rs/signal-hook/0.4](https://docs.rs/signal-hook/latest/signal_hook/index.html) — flag::register, SIGTERM constant
- src/git/rewrite.rs — cascade logic (`any_parent_remapped`) that scan must replicate

### Secondary (MEDIUM confidence)
- [ratatui.rs/tutorials/counter-app/basic-app](https://ratatui.rs/tutorials/counter-app/basic-app/) — event loop pattern, KeyEventKind::Press filter
- [ratatui.rs/concepts/event-handling](https://ratatui.rs/concepts/event-handling/) — centralised vs. distributed event handling approaches
- [crates.io/signal-hook](https://crates.io/crates/signal-hook) — 151M downloads, version history

### Tertiary (LOW confidence)
- nucleo-matcher 0.3.1 lower-level API (considered and rejected in favour of high-level nucleo API)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages verified via cargo search and docs.rs
- Architecture (scan module gap + cascade requirement): HIGH — derived from rewrite.rs source code + requirements text
- ratatui API patterns: HIGH — verified against docs.rs 0.30
- git2 signature detection: HIGH — `header_field_bytes("gpgsig")` verified on docs.rs 0.21
- nucleo integration: MEDIUM — API verified on docs.rs but tick() timing is empirical
- SIGTERM handling: HIGH — signal-hook 0.4 API verified; `flag::register` interface confirmed unchanged

**Research date:** 2026-05-20
**Valid until:** 2026-06-20 (ratatui 0.30 is current stable; no breaking changes expected in 30 days)

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Two-option main menu on launch | `Screen::MainMenu { selected }` + render; keyboard nav via `handle_key` |
| RENAME-01 | Fuzzy-filterable author list with commit counts | `nucleo::Nucleo<AuthorIdentity>`; `Screen::AuthorList`; `List` widget + `ListState` |
| RENAME-02 | Two-field free-text form for new name + email | Custom `Screen::RenameForm` with two `String` fields; `frame.set_cursor_position()` for cursor |
| RENAME-05 | Show exact affected commit count + confirm before write | `git::scan::scan_rename` with cascade tracking returns `RewritePreview.affected_count` |
| DROP-01 | Fuzzy-filterable co-author list with commit counts | Same pattern as RENAME-01 but with `CoAuthorEntry` |
| DROP-04 | Show exact affected commit count + confirm before write | `git::scan::scan_drop` with cascade tracking returns `RewritePreview.affected_count` |
| SAFE-03 | Non-blocking warning for GPG/SSH signed commits | `commit.header_field_bytes("gpgsig"/"sshsig")` on cascaded set; count in `RewritePreview.signed_commit_count` |
| SAFE-04 | Non-blocking warning for annotated tags over affected commits | Tag walk over `would_remap` set; names in `RewritePreview.annotated_tags_affected` |
| SAFE-05 | Non-blocking warning for refs/notes/commits | `note_default_ref()` + `find_reference("refs/notes/commits")`; `RewritePreview.has_notes_ref` |
| OUT-01 | Success screen: rewritten count + force-push reminder with remote name | `repo.remotes()` → prefer "origin"; `RewritePreview.remote_name`; `Screen::Success` |
</phase_requirements>
