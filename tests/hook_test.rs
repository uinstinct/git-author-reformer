mod common;

use git_author_reformer::error::AppError;
use git_author_reformer::hook::{install_strip, read_strip_list, remove_strip};
use git_author_reformer::hook::{AddResult, HookState, RemoveResult};

fn hook_path(repo: &git2::Repository) -> std::path::PathBuf {
    repo.path().join("hooks").join("commit-msg")
}

// Helper: create refs/stash without requiring working-tree WIP (mirrors preflight_test.rs:25-26).
fn create_stash_ref(repo: &git2::Repository) {
    let head_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.reference("refs/stash", head_oid, false, "test stash")
        .unwrap();
}

/// HOOK-04: Fresh install on a repo with no hook writes the file with the
/// marker block and the requested email.
#[test]
fn test_install_fresh_writes_file_with_markers_and_email() {
    let (_dir, repo) = common::create_fixture_repo();
    let result = install_strip(&repo, "bob@example.com").unwrap();
    assert!(
        matches!(result, AddResult::Installed { count: 1 }),
        "expected Installed {{ count: 1 }}, got unexpected variant"
    );

    let state = read_strip_list(&repo).unwrap();
    match state {
        HookState::Managed { emails } => {
            assert_eq!(emails, vec!["bob@example.com"]);
        }
        _ => panic!("expected HookState::Managed after fresh install"),
    }

    let contents = std::fs::read_to_string(hook_path(&repo)).unwrap();
    assert!(
        contents.contains(">>> git-author-reformer auto-strip BEGIN >>>"),
        "hook must contain BEGIN marker"
    );
    assert!(
        contents.contains("<<< git-author-reformer auto-strip END <<<"),
        "hook must contain END marker"
    );
}

/// HOOK-04: Installing a second email on a tool-managed hook appends it and
/// rewrites the file.
#[test]
fn test_install_appends_to_existing_tool_managed_hook() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();
    let result = install_strip(&repo, "carol@example.com").unwrap();
    assert!(
        matches!(result, AddResult::Installed { count: 2 }),
        "expected Installed {{ count: 2 }} after second install"
    );

    let state = read_strip_list(&repo).unwrap();
    match state {
        HookState::Managed { emails } => {
            assert_eq!(emails, vec!["bob@example.com", "carol@example.com"]);
        }
        _ => panic!("expected HookState::Managed after two installs"),
    }
}

/// HOOK-05: Adding an email already in the strip list (same case or different
/// case) is a no-op — the file bytes must not change.
#[test]
fn test_install_duplicate_email_is_noop_file_bytes_identical() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();

    let pre_bytes = std::fs::read(hook_path(&repo)).unwrap();

    // Exact duplicate.
    let result = install_strip(&repo, "bob@example.com").unwrap();
    assert!(
        matches!(result, AddResult::AlreadyStripped),
        "expected AlreadyStripped for exact duplicate"
    );
    let post_bytes = std::fs::read(hook_path(&repo)).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "file bytes must be identical after duplicate install"
    );

    // Mixed-case duplicate.
    let result2 = install_strip(&repo, "BOB@EXAMPLE.COM").unwrap();
    assert!(
        matches!(result2, AddResult::AlreadyStripped),
        "expected AlreadyStripped for mixed-case duplicate"
    );
    let post_bytes2 = std::fs::read(hook_path(&repo)).unwrap();
    assert_eq!(
        pre_bytes, post_bytes2,
        "file bytes must be identical after mixed-case duplicate install"
    );
}

/// HOOK-06: Install on a non-tool-managed hook returns Err(HookExists) and
/// leaves the existing file byte-for-byte identical.
#[test]
fn test_install_refuses_to_overwrite_non_tool_managed_hook() {
    let (_dir, repo) = common::create_fixture_repo();
    let hpath = hook_path(&repo);

    // Write a foreign hook without the tool markers.
    std::fs::create_dir_all(hpath.parent().unwrap()).unwrap();
    std::fs::write(&hpath, "#!/bin/sh\necho user hook\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&hpath, perms).unwrap();
    }

    let pre_bytes = std::fs::read(&hpath).unwrap();
    #[cfg(unix)]
    let pre_mode = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(&hpath).unwrap().permissions().mode()
    };

    let result = install_strip(&repo, "bob@example.com");
    assert!(
        matches!(result, Err(AppError::HookExists(_))),
        "expected Err(HookExists) for non-tool-managed hook"
    );

    let post_bytes = std::fs::read(&hpath).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "file bytes must be unchanged after refused install"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let post_mode = std::fs::metadata(&hpath).unwrap().permissions().mode();
        assert_eq!(pre_mode, post_mode, "file mode must be unchanged");
    }
}

/// HOOK-07: Fresh install produces a hook with mode 0755 on Unix.
#[cfg(unix)]
#[test]
fn test_install_sets_mode_0755_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();
    let mode = std::fs::metadata(hook_path(&repo))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o777,
        0o755,
        "hook file must have mode 0755 after install"
    );
}

/// HOOK-07: Generated hook starts with POSIX shebang and contains both
/// marker substrings. No CRLF line endings (Pitfall §4).
#[test]
fn test_generated_hook_has_posix_shebang_and_markers() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();
    let contents = std::fs::read_to_string(hook_path(&repo)).unwrap();
    assert!(
        contents.starts_with("#!/bin/sh\n"),
        "hook must start with '#!/bin/sh\\n'"
    );
    assert!(
        contents.contains(">>> git-author-reformer auto-strip BEGIN >>>"),
        "hook must contain BEGIN marker"
    );
    assert!(
        contents.contains("<<< git-author-reformer auto-strip END <<<"),
        "hook must contain END marker"
    );
    assert!(
        !contents.contains("\r\n"),
        "hook must not contain CRLF line endings (Pitfall §4)"
    );
}

/// HOOK-08: Generated shell script strips Co-authored-by trailers case-insensitively
/// when executed with /bin/sh. Target email appears in three trailers with varying
/// case; all three must be dropped. An unrelated co-author must be preserved.
#[test]
fn test_shell_hook_strips_case_insensitive_matches() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();

    let input = "feat: thing\n\nCo-Authored-By: Bob <BOB@EXAMPLE.COM>\nco-authored-by: Carol <carol@example.com>\nCO-AUTHORED-BY: Bob Two <bob@example.com>\n";
    let result = common::run_hook_on_message(&hook_path(&repo), input);

    // All bob@example.com trailers (any case) must be stripped.
    assert!(
        !result.to_ascii_lowercase().contains("bob@example.com"),
        "all bob@example.com trailers must be stripped; got: {result:?}"
    );
    // Carol must be preserved.
    assert!(
        result.contains("carol@example.com"),
        "carol@example.com trailer must be preserved; got: {result:?}"
    );
    // Subject line must be preserved.
    assert!(
        result.contains("feat: thing"),
        "subject line must be preserved; got: {result:?}"
    );
}

/// HOOK-08 (twin-parity counterexample — Pitfall §1 — load-bearing):
/// When the target email appears ONLY in the name slot (not the email slot),
/// the structural awk parser must preserve the line. A naive `grep -i email`
/// implementation would incorrectly drop it.
///
/// Input: `Co-authored-by: bob@example.com <alice@example.com>`
/// Target: `bob@example.com`
/// Expected: line preserved (email slot is alice@, not bob@).
#[test]
fn test_shell_hook_preserves_when_email_only_in_name_slot() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();

    let input = "feat: thing\n\nCo-authored-by: bob@example.com <alice@example.com>\n";
    let result = common::run_hook_on_message(&hook_path(&repo), input);

    assert!(
        result.contains("Co-authored-by: bob@example.com <alice@example.com>"),
        "line must be preserved when target email is only in name slot; got: {result:?}"
    );
}

/// HOOK-10: Removing a non-last entry rewrites the file with that email gone.
#[test]
fn test_remove_single_entry_rewrites_file() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();
    install_strip(&repo, "carol@example.com").unwrap();

    let result = remove_strip(&repo, "bob@example.com").unwrap();
    assert!(
        matches!(result, RemoveResult::Updated { remaining: 1 }),
        "expected Updated {{ remaining: 1 }}, got unexpected variant"
    );

    assert!(
        hook_path(&repo).exists(),
        "hook file must still exist after non-last removal"
    );

    let state = read_strip_list(&repo).unwrap();
    match state {
        HookState::Managed { emails } => {
            assert_eq!(emails, vec!["carol@example.com"]);
        }
        _ => panic!("expected Managed with one email after removal"),
    }
}

/// HOOK-10: Removing the last entry deletes the hook file entirely.
#[test]
fn test_remove_last_entry_deletes_file() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();

    let result = remove_strip(&repo, "bob@example.com").unwrap();
    assert!(
        matches!(result, RemoveResult::HookDeleted),
        "expected HookDeleted after removing last entry"
    );

    assert!(
        !hook_path(&repo).exists(),
        "hook file must not exist after last entry removed"
    );

    let state = read_strip_list(&repo).unwrap();
    assert!(
        matches!(state, HookState::Absent),
        "read_strip_list must return Absent after hook deletion"
    );
}

/// HOOK-12: Hook engine operations succeed on a repo that has a stash entry.
/// The engine must NOT call check_stash / check_worktrees internally.
#[test]
fn test_install_does_not_trigger_preflight_with_stash_present() {
    let (_dir, repo) = common::create_fixture_repo();
    create_stash_ref(&repo);

    // Confirm refs/stash exists (mirrors preflight_test.rs:25-26 pattern).
    assert!(
        repo.find_reference("refs/stash").is_ok(),
        "refs/stash must exist for this test to be meaningful"
    );

    let result = install_strip(&repo, "bob@example.com").unwrap();
    assert!(
        matches!(result, AddResult::Installed { count: 1 }),
        "install_strip must succeed even when refs/stash is present"
    );
}

/// HOOK-13: Full write→read round-trip — installing three emails and reading
/// back via read_strip_list yields the same list in insertion order.
#[test]
fn test_read_strip_list_round_trips_through_render() {
    let (_dir, repo) = common::create_fixture_repo();
    install_strip(&repo, "bob@example.com").unwrap();
    install_strip(&repo, "carol@example.com").unwrap();
    install_strip(&repo, "dave@example.com").unwrap();

    let state = read_strip_list(&repo).unwrap();
    match state {
        HookState::Managed { emails } => {
            assert_eq!(
                emails,
                vec!["bob@example.com", "carol@example.com", "dave@example.com"],
                "round-trip must preserve insertion order and all three emails"
            );
        }
        _ => panic!("expected HookState::Managed after three installs"),
    }
}
