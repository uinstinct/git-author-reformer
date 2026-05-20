mod common;

use git2::{Repository, Signature, Time};
use git_author_reformer::git::rewrite::{drop_coauthor, rewrite_author};
use git_author_reformer::git::scan::{scan_drop, scan_rename};

// ---------------------------------------------------------------------------
// Helper: build the same two-commit author + cascade fixture in a fresh repo.
// Alice authors the first commit; Bob authors a second commit whose parent is
// Alice's, so Bob's commit is in the cascade set.
// ---------------------------------------------------------------------------
fn build_alice_cascade_fixture() -> (tempfile::TempDir, Repository) {
    let (dir, repo) = common::create_fixture_repo();
    // The fixture repo already has an initial commit by Alice.
    // Add a second commit by Bob that has Alice's commit as its parent.
    common::add_commit_with_message(&repo, "Bob", "bob@example.com", "bob-downstream");
    (dir, repo)
}

// ---------------------------------------------------------------------------
// CASCADE EQUIVALENCE — RENAME-05
// ---------------------------------------------------------------------------

#[test]
fn test_scan_rename_count_matches_rewrite_author() {
    // Fixture 1: scan
    let (_dir1, repo1) = build_alice_cascade_fixture();
    let preview = scan_rename(&repo1, "Alice", "alice@example.com").unwrap();
    let scan_count = preview.affected_count;

    // Fixture 2: identical setup — rewrite (mutates repo)
    let (_dir2, repo2) = build_alice_cascade_fixture();
    let rewrite_count = rewrite_author(
        &repo2,
        "Alice",
        "alice@example.com",
        "Alice New",
        "new@example.com",
    )
    .unwrap();

    assert_eq!(
        scan_count,
        rewrite_count,
        "RENAME-05 requires the count shown at confirmation to equal the count actually rewritten — cascade tracking is mandatory. scan={scan_count}, rewrite={rewrite_count}"
    );
}

#[test]
fn test_scan_rename_counts_cascade_descendants() {
    // Alice authors commit A; Bob authors commit B with A as parent.
    // Scanning for Alice must report affected_count == 2 (A + B's cascade).
    let (_dir, repo) = build_alice_cascade_fixture();
    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();
    assert_eq!(
        preview.affected_count,
        2,
        "RENAME-05 / Pitfall 2: cascade descendants must be counted — Bob's commit is in the cascade set because its parent (Alice's commit) would be remapped. got: {}",
        preview.affected_count
    );
}

#[test]
fn test_scan_rename_zero_when_no_match() {
    let (_dir, repo) = common::create_fixture_repo();
    let preview = scan_rename(&repo, "Nobody", "nobody@example.com").unwrap();
    assert_eq!(
        preview.affected_count, 0,
        "RENAME-05: scanning for an identity with no commits must return affected_count == 0"
    );
    assert_eq!(
        preview.signed_commit_count, 0,
        "RENAME-05: scanning for an identity with no commits must return signed_commit_count == 0"
    );
    assert!(
        preview.annotated_tags_affected.is_empty(),
        "RENAME-05: scanning for an identity with no commits must return empty annotated_tags_affected"
    );
}

// ---------------------------------------------------------------------------
// CASCADE EQUIVALENCE — DROP-04
// ---------------------------------------------------------------------------

#[test]
fn test_scan_drop_count_matches_drop_coauthor() {
    // Build two identical fixtures with co-authored commits.
    let build_drop_fixture = || {
        let (dir, repo) = common::create_fixture_repo();
        common::add_commit_with_message(
            &repo,
            "Alice",
            "alice@example.com",
            "feat: x\n\nCo-authored-by: Bob <bob@example.com>\n",
        );
        // Bob's downstream commit cascades from Alice's co-authored commit.
        common::add_commit_with_message(&repo, "Carol", "carol@example.com", "carol-downstream");
        (dir, repo)
    };

    let (_dir1, repo1) = build_drop_fixture();
    let preview = scan_drop(&repo1, "bob@example.com").unwrap();
    let scan_count = preview.affected_count;

    let (_dir2, repo2) = build_drop_fixture();
    let rewrite_count = drop_coauthor(&repo2, "bob@example.com").unwrap();

    assert_eq!(
        scan_count,
        rewrite_count,
        "DROP-04 requires the count shown at confirmation to equal the count actually rewritten — cascade tracking is mandatory. scan={scan_count}, rewrite={rewrite_count}"
    );
}

// ---------------------------------------------------------------------------
// SIGNATURE DETECTION — SAFE-03
// ---------------------------------------------------------------------------

#[test]
fn test_scan_rename_counts_signed_commits_in_cascade_only() {
    let (_dir, repo) = common::create_fixture_repo();

    // Create a signed commit by Alice using commit_create_buffer + commit_signed.
    // This commit is in the cascade set for "Alice".
    let sig = Signature::new("Alice", "alice@example.com", &Time::new(2_000_000, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    let buf = repo
        .commit_create_buffer(&sig, &sig, "alice-signed", &tree, &[&head])
        .unwrap();
    let commit_content = std::str::from_utf8(&buf).unwrap();
    let fake_gpgsig = "-----BEGIN PGP SIGNATURE-----\nfakesig\n-----END PGP SIGNATURE-----";
    let signed_oid = repo
        .commit_signed(commit_content, fake_gpgsig, Some("gpgsig"))
        .unwrap();

    // Point HEAD at the signed commit so revwalk finds it.
    repo.reference("refs/heads/master", signed_oid, true, "update")
        .unwrap();

    // Create an orphan signed commit by Bob — no parents, not by Alice.
    // This commit IS reachable (on refs/heads/bob-branch) so revwalk visits it,
    // but it is NOT in Alice's cascade set because:
    //   (a) Bob is not Alice → identity_matches = false
    //   (b) Bob has no parents → any_parent_remapped = false
    // If count_signed_commits stopped filtering by would_remap, it would count 2.
    let bob_sig = Signature::new("Bob", "bob@example.com", &Time::new(2_000_001, 0)).unwrap();
    let bob_tree_oid = {
        let mut idx = repo.index().unwrap();
        idx.write_tree().unwrap()
    };
    let bob_tree = repo.find_tree(bob_tree_oid).unwrap();
    let bob_buf = repo
        .commit_create_buffer(&bob_sig, &bob_sig, "bob-orphan-signed", &bob_tree, &[])
        .unwrap();
    let bob_content = std::str::from_utf8(&bob_buf).unwrap();
    let bob_signed_oid = repo
        .commit_signed(bob_content, fake_gpgsig, Some("gpgsig"))
        .unwrap();
    repo.reference("refs/heads/bob-branch", bob_signed_oid, false, "bob orphan")
        .unwrap();

    // Verify: Alice's signed commit has gpgsig header.
    let alice_signed_commit = repo.find_commit(signed_oid).unwrap();
    assert!(
        alice_signed_commit.header_field_bytes("gpgsig").is_ok(),
        "SAFE-03: precondition — Alice's commit must have gpgsig header after commit_signed"
    );

    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();
    assert_eq!(
        preview.signed_commit_count,
        1,
        "SAFE-03 warning must reflect only the cascade set, not the whole repo — Alice's signed commit is in cascade, Bob's is not (or not signed commits outside cascade). got: {}",
        preview.signed_commit_count
    );
}

// ---------------------------------------------------------------------------
// ANNOTATED TAG DETECTION — SAFE-04
// ---------------------------------------------------------------------------

#[test]
fn test_scan_rename_lists_annotated_tags_pointing_at_cascade() {
    let (_dir, repo) = common::create_fixture_repo();

    // The fixture repo's initial commit (by Alice) is the HEAD.
    let alice_commit = repo.head().unwrap().peel_to_commit().unwrap();

    // Create annotated tag "v1.0" pointing at Alice's commit.
    common::create_annotated_tag(&repo, "v1.0", &alice_commit, "release v1.0");

    // Add a lightweight tag pointing at Alice's commit (should NOT appear in results).
    repo.reference(
        "refs/tags/lightweight-tag",
        alice_commit.id(),
        false,
        "lw tag",
    )
    .unwrap();

    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();

    assert!(
        preview.annotated_tags_affected.contains(&"v1.0".to_string()),
        "RENAME-04 + SAFE-04: annotated tag 'v1.0' pointing at a cascade commit must appear in annotated_tags_affected. got: {:?}",
        preview.annotated_tags_affected
    );

    assert!(
        !preview
            .annotated_tags_affected
            .contains(&"lightweight-tag".to_string()),
        "SAFE-04: lightweight tags must NOT appear in annotated_tags_affected (only annotated tag objects require recreation). got: {:?}",
        preview.annotated_tags_affected
    );
}

// ---------------------------------------------------------------------------
// NOTES REF DETECTION — SAFE-05
// ---------------------------------------------------------------------------

#[test]
fn test_scan_drop_detects_notes_ref_when_present() {
    let (_dir, repo) = common::create_fixture_repo();

    // Add a co-authored commit so scan_drop has something to scan.
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat\n\nCo-authored-by: Bob <bob@example.com>\n",
    );

    // Create a note on the initial commit.
    let commit_oid = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent_id(0)
        .unwrap();
    let note_sig = Signature::new("Tester", "test@example.com", &Time::new(3_000_000, 0)).unwrap();
    repo.note(&note_sig, &note_sig, None, commit_oid, "note body", false)
        .unwrap();

    let preview = scan_drop(&repo, "bob@example.com").unwrap();
    assert!(
        preview.has_notes_ref,
        "SAFE-05: has_notes_ref must be true when refs/notes/commits exists in the repo"
    );
}

#[test]
fn test_scan_drop_no_notes_ref_when_absent() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat\n\nCo-authored-by: Bob <bob@example.com>\n",
    );

    let preview = scan_drop(&repo, "bob@example.com").unwrap();
    assert!(
        !preview.has_notes_ref,
        "SAFE-05: has_notes_ref must be false when no refs/notes/commits exists in the repo"
    );
}

// ---------------------------------------------------------------------------
// REMOTE DETECTION — OUT-01
// ---------------------------------------------------------------------------

#[test]
fn test_scan_rename_prefers_origin_remote() {
    let (_dir, repo) = common::create_fixture_repo();
    // Add two remotes; "origin" should be preferred.
    repo.remote("upstream", "https://example.com/upstream.git")
        .unwrap();
    repo.remote("origin", "https://example.com/origin.git")
        .unwrap();

    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();
    assert_eq!(
        preview.remote_name,
        Some("origin".to_string()),
        "OUT-01: remote_name must be Some(\"origin\") when origin remote exists, regardless of insertion order. got: {:?}",
        preview.remote_name
    );
}

#[test]
fn test_scan_rename_first_remote_when_no_origin() {
    let (_dir, repo) = common::create_fixture_repo();
    repo.remote("upstream", "https://example.com/upstream.git")
        .unwrap();

    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();
    assert_eq!(
        preview.remote_name,
        Some("upstream".to_string()),
        "OUT-01: remote_name must be Some(\"upstream\") when only upstream remote exists and no origin. got: {:?}",
        preview.remote_name
    );
}

#[test]
fn test_scan_rename_none_when_no_remote() {
    let (_dir, repo) = common::create_fixture_repo();
    // Fresh fixture repo has no remotes.
    let preview = scan_rename(&repo, "Alice", "alice@example.com").unwrap();
    assert_eq!(
        preview.remote_name, None,
        "OUT-01: remote_name must be None when no remotes are configured. got: {:?}",
        preview.remote_name
    );
}
