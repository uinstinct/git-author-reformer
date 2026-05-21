---
phase: 05-hook-engine
reviewed: 2026-05-21T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - src/hook/mod.rs
  - src/hook/parse.rs
  - src/hook/render.rs
  - src/hook/write.rs
  - src/hook/path.rs
  - src/error.rs
  - tests/hook_test.rs
  - tests/common/mod.rs
findings:
  critical: 1
  warning: 2
  info: 2
  total: 5
fixes_applied:
  critical: 1
  warning: 2
  info: 0
  total: 3
status: fixed_critical_and_warning
remaining: 2 info findings (IN-01 symlink in delete_hook, IN-02 marker collision) — accepted as known tradeoffs
---

# Phase 05: Code Review Report

**Reviewed:** 2026-05-21
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

The hook engine is well-structured and the twin-parity analysis between the awk filter and the Rust drop-flow checks out (see dedicated section below). The atomic write pattern is correct: permissions are set on the tmp file before rename, CRLF is absent, and mode 0755 is applied on Unix. Marker detection correctly requires both BEGIN and END in order.

One critical shell-injection bug was found: the email validator does not reject the single-quote character, but the awk program is embedded inside shell single-quotes. An email containing `'` terminates the awk string literal prematurely and allows injection of arbitrary shell commands when the generated hook runs. This must be fixed before the hook engine is wired into the TUI.

A secondary gap exists on the read path: existing emails recovered from disk are passed directly to `render_hook` without re-validation, creating a defense-in-depth hole if the hook file has been hand-edited.

---

## Critical Issues

### CR-01: Single-quote not rejected by email validator — shell injection in generated hook

**File:** `src/hook/render.rs:76`

**Issue:**
`validate_email_for_embedding` rejects `"`, `\`, `\n`, and `\r`, but does NOT reject the single-quote character (`'`). The generated hook wraps the entire awk program in shell single-quotes (render.rs:39, 62):

```sh
awk '
BEGIN {
  strip["alice@example.com"] = 1
}
...
' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
```

An email containing `'` is embedded at line 41 via `format!("  strip[\"{}\"] = 1\n", e)`. Consider an email `x'] }' ; id ; awk '` accepted by the current validator — after lowercasing it becomes:

```sh
awk '
BEGIN {
  strip["x'] }' ; id ; awk '"] = 1
}
...
' "$1"
```

The shell parses this as: first `awk '...'` closed at the embedded `'`, then `; id ;` executes as a shell command, then a new `awk '` begins — which is a shell parse error in this specific string, but with a more carefully crafted payload the injection is fully exploitable. The RESEARCH §Security Domain acknowledges `'` is structurally safe for the `strip["..."]` lookup itself, but the threat is at the shell-quoting layer, not the awk layer. Any email containing a `'` terminates the shell's single-quote context prematurely.

**Real-world trigger:** `git2::Signature::email()` can return arbitrary byte sequences. A commit message authored by someone with a single-quote in their email address (uncommon but RFC-5321-legal in the local part if quoted) would produce a valid co-author entry that, when the user selects it from the TUI list and calls `install_strip`, writes a broken or injecting hook.

**Fix:** Add `'\''` (single-quote escape) to the rejected set, or escape single-quotes in the embedded email. The simplest safe fix is to extend the validator:

```rust
pub(crate) fn validate_email_for_embedding(email: &str) -> Result<(), &'static str> {
    for ch in email.chars() {
        match ch {
            '"' | '\\' | '\n' | '\r' | '\'' => {
                return Err("email contains forbidden character for awk embedding")
            }
            _ => {}
        }
    }
    Ok(())
}
```

Alternatively, if single-quote rejection is too broad (theoretically valid in quoted local-part per RFC 5321), use a double-quote-delimited awk string and pass it via `-v` or a temp variable instead of shell inline — but the simplest correct fix for this codebase is the rejection above. Real email addresses do not contain `'` in practice.

Also add a test:

```rust
#[test]
fn validate_email_for_embedding_rejects_single_quote() {
    let result = validate_email_for_embedding("it'shim@x.com");
    assert!(result.is_err(), "single quote in email must be rejected");
}
```

---

## Warnings

### WR-01: Emails read from disk are not re-validated before re-rendering

**File:** `src/hook/mod.rs:65,71`

**Issue:**
`install_strip` and `remove_strip` both call `read_strip_list(repo)` (which calls `extract_strip_list`), obtain `emails: Vec<String>` from disk, and then pass that entire list directly to `render_hook`. Only the single _new_ email from the caller is validated (mod.rs:54). Emails already on disk are trusted unconditionally.

If the single-quote fix from CR-01 is applied to the validator but the hook file is hand-edited to insert a dangerous email (or the file was written before a future fix is deployed), the next `install_strip` or `remove_strip` call will re-render those poisoned entries verbatim, regenerating the vulnerable hook.

```rust
// mod.rs:62-71 — only `email` (the new entry) is validated; `emails` from disk is not
let mut emails = match read_strip_list(repo)? {
    ...
    HookState::Managed { emails } => emails,   // <-- not validated
};
...
write::atomic_write_executable(&hook_path, &render::render_hook(&emails))?;
```

**Fix:** Validate all entries before rendering. One approach: add a validation pass after `extract_strip_list` returns, or validate inside `render_hook` itself (and return a `Result`) to make it structurally impossible to render unvalidated content:

```rust
// In install_strip, after reading existing emails:
let HookState::Managed { emails } = ... else { ... };
for e in &emails {
    if render::validate_email_for_embedding(e).is_err() {
        return Err(AppError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("hook file contains an email with forbidden characters: {e:?}"),
        )));
    }
}
```

Alternatively, validate inside `extract_strip_list` (returning only valid entries and logging a warning for malformed ones) so no downstream caller can receive an unvalidated email.

### WR-02: Input validation error wrapped in wrong error type with insufficient detail

**File:** `src/hook/mod.rs:55-58`

**Issue:**
When an email fails validation, `install_strip` returns:

```rust
Err(crate::error::AppError::Io(io::Error::new(
    io::ErrorKind::InvalidInput,
    "email is empty or contains characters forbidden for hook embedding",
)))
```

This wraps an input validation failure in `AppError::Io`, which is the type derived from `std::io::Error` (for filesystem operations). Callers and error reporters that pattern-match on `AppError::Io` will conflate a filesystem error with a business-logic validation failure. The `thiserror`-based `AppError` enum is already used for domain-level error discrimination; this should be a distinct variant.

Additionally, the error message does not name which character is forbidden or what the email value was, making it harder for a Phase 6 TUI to render a useful diagnostic.

**Fix:** Add a `HookInvalidEmail(String)` variant to `AppError` in `src/error.rs`:

```rust
#[error("Email {0:?} contains a character not allowed in the hook strip list (forbidden: \", \\, ', newline).")]
HookInvalidEmail(String),
```

Then in `install_strip`:

```rust
if email.is_empty() || render::validate_email_for_embedding(email).is_err() {
    return Err(crate::error::AppError::HookInvalidEmail(email.to_string()));
}
```

---

## Info

### IN-01: `delete_hook` does not guard against symlinks

**File:** `src/hook/write.rs:38-40`

**Issue:**
`delete_hook` calls `fs::remove_file(path)` unconditionally. On Unix, `remove_file` on a symlink removes the symlink itself (not the target), which is correct POSIX behavior. However, if the hook path happens to be a symlink to a shared resource (e.g., a hooks template directory mounted read-only), the delete call could silently succeed (removing the symlink) while the caller believes the hook is gone — but a subsequent `fs::read_to_string` check on the directory target would still find content.

The practical risk is low given this tool's scope (hooks directory is user-controlled), and the RESEARCH §Security Domain notes that the threat model does not include adversarial filesystem state. Flagged for awareness.

**Fix (optional):** A lightweight guard would check `fs::symlink_metadata(path)?.file_type().is_symlink()` before deletion and return an error, consistent with the refuse-to-overwrite philosophy in HOOK-06. Whether this is worth the complexity is a judgment call for v1.1.

### IN-02: Marker collision is undetected if user pastes the BEGIN marker into a commit message

**File:** `src/hook/parse.rs:14-21`

**Issue:**
The marker detection uses `str::find` to locate `BEGIN_MARKER` and `END_MARKER` anywhere in the hook file. If a user's pre-existing hook file contained a comment that exactly matches `BEGIN_MARKER` (e.g., copied from this tool's README), `detect_markers` would classify the hook as tool-managed and `read_strip_list` would return `HookState::Managed` — potentially allowing `install_strip` to silently overwrite a foreign hook.

This is Pitfall §2 from RESEARCH.md and the design acknowledges it as an acceptable low-probability risk given the specificity of the marker strings (`git-author-reformer auto-strip BEGIN` with directional chevrons). The two-marker requirement (both BEGIN and END) significantly reduces the collision surface. No code change is required; documented here as a known tradeoff.

**Fix:** No immediate fix needed. If desired, add a format version prefix (e.g., `# >>> git-author-reformer auto-strip BEGIN v1 >>>`) to future-proof against accidental matches from docs-copy-paste.

---

## Twin-Parity Analysis (HOOK-08): awk vs Rust drop-flow

Performed against `src/git/reader.rs:84-107` (Rust source of truth) and `src/hook/render.rs:39-62` (awk twin).

| Step | Rust (`reader.rs`) | awk (`render.rs`) | Verdict |
|------|--------------------|-------------------|---------|
| Leading whitespace trim before prefix check | `line.trim()` at reader.rs:49 (via `trimmed` in `enumerate_coauthors`) | `sub(/^[ \t]+/, "", t)` at render.rs:45 | Matches — both strip only leading whitespace |
| Prefix match ("co-authored-by:", 15 chars) | `slice.eq_ignore_ascii_case(prefix)` reader.rs:87 | `tolower(substr(t, 1, 15)) != "co-authored-by:"` render.rs:46-47 | Matches — both ASCII case-fold over exactly 15 chars |
| Structural email extraction: rightmost `<` and `>` | `value.rfind('<')` / `value.rfind('>')` reader.rs:96-97 | Backwards `for` loop from `length(rest)` to `1`, first `>` found sets `gt`, then scans back for `<` render.rs:50-54 | Matches — both find the _last_ `<` and the _last_ `>` |
| Rejection when `>` precedes `<` | `if gt < lt { return None }` reader.rs:98 | `if (lt == 0 \|\| gt == 0 \|\| gt < lt) { print; next }` render.rs:55 | Matches |
| Email trim | `value[lt+1..gt].trim()` reader.rs:102 | `gsub(/^[ \t]+\|[ \t]+$/, "", email)` render.rs:57 | Matches — both strip leading/trailing whitespace from extracted email |
| Case-fold before compare | `email.eq_ignore_ascii_case(target)` rewrite.rs:190 | `email = tolower(email)` + hash lookup render.rs:58-59 | Matches — both ASCII case-fold only (not Unicode); deliberate and symmetric |
| Empty-both-name-and-email rejection | `if name.is_empty() && email.is_empty() { return None }` reader.rs:103-105 | Not present in awk | **Minor divergence** — awk would look up an empty string `""` in the strip hash and not find it (unless `""` is an entry, which cannot happen via `install_strip`), then print the line. Effective behavior is the same: the line is preserved. Not a correctness defect under current constraints, but the Rust-side semantic is not fully mirrored. |

**Conclusion:** Twin-parity is substantively correct. The single semantic gap (empty name+email rejection) does not affect runtime behavior because the strip hash never contains an empty-string entry. No defect is raised; noted for documentation completeness.

---

_Reviewed: 2026-05-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
