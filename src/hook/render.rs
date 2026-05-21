//! Hook file renderer — produces the full POSIX sh hook body from a strip email list.
//!
//! The rendered hook embeds the strip list twice:
//! 1. As `# <email>` comment lines between the BEGIN/END markers (for the Rust parser to read back).
//! 2. As `strip["<email>"] = 1` entries in the awk BEGIN block (for the runtime filter).
//!
//! This is a pure function — no I/O. Plan 04 (write.rs) does the filesystem work.

use crate::hook::parse::{BEGIN_MARKER, END_MARKER};

/// Render the full POSIX sh hook file body for a given strip list.
///
/// Emails are lowercased before embedding. All inputs MUST be pre-validated
/// with `validate_email_for_embedding` (called by `install_strip` upstream).
///
/// The output uses LF (`\n`) line endings exclusively — never CRLF (Pitfall §4).
pub(crate) fn render_hook(emails: &[String]) -> String {
    let lowercased: Vec<String> = emails.iter().map(|e| e.to_ascii_lowercase()).collect();

    // Build comment block (for Rust parser — HOOK-08 twin parity source)
    let comment_lines: String = lowercased.iter().map(|e| format!("# {}\n", e)).collect();

    // Build awk strip array entries (for runtime filter)
    let awk_strip_entries: String = lowercased
        .iter()
        .map(|e| format!("  strip[\"{}\"] = 1\n", e))
        .collect();

    format!(
        r#"#!/bin/sh
{begin}
{comment_lines}{end}

# Filter Co-authored-by trailers whose email matches any in the embedded list.
# Twin of the Rust drop flow in src/git/reader.rs + src/git/rewrite.rs.
# Matching semantics: case-insensitive prefix on "co-authored-by:", structural
# extraction of email from the LAST <...> pair, ASCII case-fold compare.

awk '
BEGIN {{
{awk_strip_entries}}}
{{
  line = $0
  t = line
  sub(/^[ \t]+/, "", t)
  prefix = tolower(substr(t, 1, 15))
  if (prefix != "co-authored-by:") {{ print; next }}
  rest = substr(t, 16)
  lt = 0; gt = 0
  for (i = length(rest); i > 0; i--) {{
    c = substr(rest, i, 1)
    if (gt == 0 && c == ">") gt = i
    if (c == "<")            {{ lt = i; break }}
  }}
  if (lt == 0 || gt == 0 || gt < lt) {{ print; next }}
  email = substr(rest, lt + 1, gt - lt - 1)
  gsub(/^[ \t]+|[ \t]+$/, "", email)
  email = tolower(email)
  if (email in strip) next
  print
}}
' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
"#,
        begin = BEGIN_MARKER,
        end = END_MARKER,
        comment_lines = comment_lines,
        awk_strip_entries = awk_strip_entries,
    )
}

/// Returns `Err` if any character in the email would break the awk string literal
/// (`"`, `\`, `\n`, `\r`). Used by `install_strip` for validation before rendering.
pub(crate) fn validate_email_for_embedding(email: &str) -> Result<(), &'static str> {
    for ch in email.chars() {
        match ch {
            '"' | '\\' | '\n' | '\r' => {
                return Err("email contains forbidden character for awk embedding")
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Shape tests ---

    #[test]
    fn render_hook_starts_with_posix_shebang_lf() {
        // HOOK-07: shebang must be "#!/bin/sh\n" with LF, never CRLF
        let out = render_hook(&["bob@example.com".to_string()]);
        assert!(
            out.starts_with("#!/bin/sh\n"),
            "output must start with '#!/bin/sh\\n'"
        );
        assert!(
            !out.contains("\r\n"),
            "output must not contain CRLF (Pitfall §4)"
        );
    }

    #[test]
    fn render_hook_contains_both_markers_in_order() {
        let out = render_hook(&["bob@example.com".to_string()]);
        let begin_pos = out
            .find(BEGIN_MARKER)
            .expect("BEGIN_MARKER must be present");
        let end_pos = out.find(END_MARKER).expect("END_MARKER must be present");
        assert!(
            begin_pos < end_pos,
            "BEGIN_MARKER must appear before END_MARKER"
        );
    }

    #[test]
    fn render_hook_embeds_email_as_comment_line() {
        // Comment line for Rust parser: "# <email>" between the two markers
        let out = render_hook(&["bob@example.com".to_string()]);
        assert!(
            out.contains("\n# bob@example.com\n"),
            "output must contain '\\n# bob@example.com\\n'"
        );
    }

    #[test]
    fn render_hook_embeds_email_in_awk_strip_array() {
        // awk array entry for runtime filter
        let out = render_hook(&["bob@example.com".to_string()]);
        assert!(
            out.contains("strip[\"bob@example.com\"] = 1"),
            "output must contain awk strip array entry"
        );
    }

    #[test]
    fn render_hook_preserves_order_for_multiple_emails() {
        let emails: Vec<String> = vec!["zebra@x.com".into(), "alpha@x.com".into()];
        let out = render_hook(&emails);

        // Comment block order
        let z_comment = out.find("# zebra@x.com").expect("zebra comment must exist");
        let a_comment = out.find("# alpha@x.com").expect("alpha comment must exist");
        assert!(z_comment < a_comment, "comments must be in input order");

        // awk array entry order
        let z_awk = out
            .find("strip[\"zebra@x.com\"]")
            .expect("zebra awk entry must exist");
        let a_awk = out
            .find("strip[\"alpha@x.com\"]")
            .expect("alpha awk entry must exist");
        assert!(z_awk < a_awk, "awk entries must be in input order");
    }

    #[test]
    fn render_hook_emits_empty_marker_block_when_list_is_empty() {
        let out = render_hook(&[]);
        assert!(
            out.contains(BEGIN_MARKER),
            "BEGIN_MARKER must be present even for empty list"
        );
        assert!(
            out.contains(END_MARKER),
            "END_MARKER must be present even for empty list"
        );
        // No comment email lines and no awk strip entries
        let begin_pos = out.find(BEGIN_MARKER).unwrap();
        let end_pos = out.find(END_MARKER).unwrap();
        // The only content between markers should be a newline (no "# email" lines)
        let between = &out[begin_pos + BEGIN_MARKER.len()..end_pos];
        assert!(
            !between.contains("# "),
            "no email comment lines between markers for empty list"
        );
        assert!(
            !out.contains("strip[\""),
            "no awk strip entries for empty list"
        );
    }

    // --- Lowercasing tests (HOOK-08 twin parity) ---

    #[test]
    fn render_hook_lowercases_emails_in_comments() {
        let out = render_hook(&["BOB@Example.COM".to_string()]);
        assert!(
            out.contains("# bob@example.com"),
            "comment must be lowercase"
        );
        assert!(
            !out.contains("BOB@Example.COM"),
            "mixed-case original must not appear"
        );
    }

    #[test]
    fn render_hook_lowercases_emails_in_awk_array() {
        let out = render_hook(&["BOB@Example.COM".to_string()]);
        assert!(
            out.contains("strip[\"bob@example.com\"] = 1"),
            "awk entry must be lowercase"
        );
        assert!(
            !out.contains("strip[\"BOB@Example.COM\"]"),
            "mixed-case original must not appear in awk entry"
        );
    }

    // --- POSIX-portability tests (HOOK-07 + Pitfall §5) ---

    #[test]
    fn render_hook_contains_no_bash_isms() {
        let out = render_hook(&["bob@example.com".to_string()]);
        // These patterns indicate bash-specific syntax not valid in POSIX sh
        assert!(!out.contains("[["), "must not use [[ (bash-ism)");
        assert!(!out.contains("]]"), "must not use ]] (bash-ism)");
        // ${...} inside single-quoted awk blocks is fine (not shell-evaluated),
        // but the template must not emit it outside — test that the raw string
        // doesn't contain ${ at all since our template uses $0/$1 not ${...}
        assert!(
            !out.contains("${"),
            "must not use ${{...}} expansion (bash-ism)"
        );
        assert!(
            !out.contains(" function "),
            "must not use 'function' keyword (bash-ism)"
        );
        assert!(
            !out.contains("printf"),
            "must not use printf (not needed, not POSIX-mandated for hooks)"
        );
    }

    #[test]
    fn render_hook_uses_awk_tolower_for_case_fold() {
        // Mirror of eq_ignore_ascii_case in src/git/rewrite.rs:190
        let out = render_hook(&["bob@example.com".to_string()]);
        assert!(
            out.contains("tolower("),
            "awk script must use tolower() for case-insensitive matching"
        );
    }

    #[test]
    fn render_hook_uses_structural_email_extraction() {
        // Mirror of parse_coauthor_value's rfind('<') / rfind('>') logic
        // The awk equivalent is a backwards for-loop using substr()
        let out = render_hook(&["bob@example.com".to_string()]);
        assert!(
            out.contains("substr("),
            "awk must use substr() for structural extraction"
        );
        assert!(
            out.contains("for ("),
            "awk must use a for loop for backwards < > scan (rfind equivalent)"
        );
    }

    // --- Validation tests (RESEARCH §Security Domain) ---

    #[test]
    fn validate_email_for_embedding_rejects_double_quote() {
        let result = validate_email_for_embedding("bob\"; rm -rf / #@x.com");
        assert!(result.is_err(), "double quote in email must be rejected");
    }

    #[test]
    fn validate_email_for_embedding_rejects_backslash() {
        let result = validate_email_for_embedding("bob\\@x.com");
        assert!(result.is_err(), "backslash in email must be rejected");
    }

    #[test]
    fn validate_email_for_embedding_rejects_newline() {
        let result = validate_email_for_embedding("bob\n@x.com");
        assert!(result.is_err(), "newline in email must be rejected");
    }

    #[test]
    fn validate_email_for_embedding_accepts_normal_email() {
        let result = validate_email_for_embedding("bob@example.com");
        assert!(result.is_ok(), "normal email must be accepted");
    }
}
