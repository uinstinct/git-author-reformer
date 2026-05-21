//! Hook file parser: marker detection and strip-list extraction.
//! Shared marker constants are `pub(crate)` so `render.rs` (Plan 03) reuses them.

/// Sentinel marking the start of the tool-managed strip region.
pub(crate) const BEGIN_MARKER: &str = "# >>> git-author-reformer auto-strip BEGIN >>>";

/// Sentinel marking the end of the tool-managed strip region.
pub(crate) const END_MARKER: &str = "# <<< git-author-reformer auto-strip END <<<";

/// Returns the byte offsets `(begin, end)` where `begin` is the start of the
/// `BEGIN_MARKER` line and `end` is the start of the `END_MARKER` line.
///
/// Returns `None` when either marker is absent or END comes before BEGIN.
pub(crate) fn detect_markers(contents: &str) -> Option<(usize, usize)> {
    let begin = contents.find(BEGIN_MARKER)?;
    let end = contents.find(END_MARKER)?;
    if end <= begin {
        return None;
    }
    Some((begin, end))
}

/// Given hook-file contents known to contain both markers, returns the list of
/// emails between them. Strips the leading `# ` prefix from each line. Skips
/// blank lines. Preserves casing as written (lowercasing is the renderer's job).
pub(crate) fn extract_strip_list(contents: &str) -> Vec<String> {
    let Some((begin, end)) = detect_markers(contents) else {
        return vec![];
    };
    // Slice from start of BEGIN marker to start of END marker.
    let region = &contents[begin..end];
    region
        .lines()
        .skip(1) // skip the BEGIN_MARKER line itself
        .filter_map(|line| {
            let stripped = line.strip_prefix("# ")?;
            let trimmed = stripped.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Full hook file template body (mirrors RESEARCH §Code Examples "Hook file template").
    fn full_hook_template() -> String {
        format!(
            "#!/bin/sh\n{}\n# bob@example.com\n# carol@example.com\n{}\n\n# Filter Co-authored-by trailers.\n",
            BEGIN_MARKER,
            END_MARKER,
        )
    }

    #[test]
    fn detect_markers_returns_some_when_both_markers_present_in_order() {
        let input = full_hook_template();
        let result = detect_markers(&input);
        assert!(
            result.is_some(),
            "expected Some when both markers present in order"
        );
        let (begin_byte, end_byte) = result.unwrap();
        assert!(begin_byte < end_byte, "begin_byte must precede end_byte");
    }

    #[test]
    fn detect_markers_returns_none_when_neither_marker_present() {
        let input = "#!/bin/sh\necho hi\n";
        let result = detect_markers(input);
        assert!(
            result.is_none(),
            "expected None for plain hook with no markers"
        );
    }

    #[test]
    fn detect_markers_returns_none_when_only_begin_marker_present() {
        let input = format!("#!/bin/sh\n{}\necho hi\n", BEGIN_MARKER);
        let result = detect_markers(&input);
        assert!(
            result.is_none(),
            "expected None when only BEGIN marker present — Pitfall §2"
        );
    }

    #[test]
    fn detect_markers_returns_none_when_only_end_marker_present() {
        let input = format!("#!/bin/sh\necho hi\n{}\n", END_MARKER);
        let result = detect_markers(&input);
        assert!(
            result.is_none(),
            "expected None when only END marker present — Pitfall §2"
        );
    }

    #[test]
    fn detect_markers_returns_none_when_end_marker_precedes_begin() {
        let input = format!("#!/bin/sh\n{}\n# email\n{}\n", END_MARKER, BEGIN_MARKER);
        let result = detect_markers(&input);
        assert!(
            result.is_none(),
            "expected None when END marker comes before BEGIN marker"
        );
    }

    #[test]
    fn extract_strip_list_returns_emails_from_comment_lines() {
        let input = format!(
            "#!/bin/sh\n{}\n# bob@example.com\n# carol@example.com\n{}\n",
            BEGIN_MARKER, END_MARKER,
        );
        let result = extract_strip_list(&input);
        assert_eq!(result, vec!["bob@example.com", "carol@example.com"]);
    }

    #[test]
    fn extract_strip_list_strips_leading_comment_hash_and_space() {
        let input = format!(
            "#!/bin/sh\n{}\n# alice@example.com\n{}\n",
            BEGIN_MARKER, END_MARKER,
        );
        let result = extract_strip_list(&input);
        assert_eq!(result, vec!["alice@example.com"]);
        // Verify the "# " prefix (hash + space) is removed, not just "#"
        assert!(!result[0].starts_with('#'), "must not retain leading '#'");
        assert!(!result[0].starts_with(' '), "must not retain leading space");
    }

    #[test]
    fn extract_strip_list_skips_blank_lines_between_markers() {
        let input = format!(
            "#!/bin/sh\n{}\n# bob@example.com\n\n# carol@example.com\n{}\n",
            BEGIN_MARKER, END_MARKER,
        );
        let result = extract_strip_list(&input);
        assert_eq!(result, vec!["bob@example.com", "carol@example.com"]);
    }

    #[test]
    fn extract_strip_list_returns_empty_when_no_emails_between_markers() {
        // Only blank lines between markers — still tool-managed, but no emails.
        let input = format!("#!/bin/sh\n{}\n\n\n{}\n", BEGIN_MARKER, END_MARKER,);
        let result = extract_strip_list(&input);
        let empty: Vec<String> = vec![];
        assert_eq!(
            result, empty,
            "expected empty list when no email lines between markers"
        );
    }

    #[test]
    fn marker_constants_are_distinctive() {
        assert!(
            BEGIN_MARKER.contains("git-author-reformer"),
            "BEGIN_MARKER must contain 'git-author-reformer'"
        );
        assert!(
            BEGIN_MARKER.chars().filter(|&c| c == '>').count() >= 3,
            "BEGIN_MARKER must contain at least three '>' chars"
        );
        assert!(
            END_MARKER.contains("git-author-reformer"),
            "END_MARKER must contain 'git-author-reformer'"
        );
        assert!(
            END_MARKER.chars().filter(|&c| c == '<').count() >= 3,
            "END_MARKER must contain at least three '<' chars"
        );
    }
}
