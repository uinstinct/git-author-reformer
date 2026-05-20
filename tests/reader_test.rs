mod common;

use git_author_reformer::git::reader::{enumerate_authors, enumerate_coauthors};
use git_author_reformer::git::types::{AuthorIdentity, CoAuthorEntry};

#[test]
fn test_enumerate_authors_empty_repo_returns_empty_vec() {
    let dir = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let result = enumerate_authors(&repo).unwrap();
    assert_eq!(result, Vec::<AuthorIdentity>::new());
}

#[test]
fn test_enumerate_authors_counts_and_sorts_descending() {
    let (_dir, repo) = common::create_fixture_repo(); // Alice: 1 commit (initial)
    common::add_commit_with_message(&repo, "Bob", "bob@example.com", "Bob's commit");
    common::add_commit_with_message(&repo, "Bob", "bob@example.com", "Bob's second commit");

    let result = enumerate_authors(&repo).unwrap();
    assert_eq!(result.len(), 2, "Expected Alice and Bob");
    assert_eq!(result[0].name, "Bob", "Bob should be first (2 commits)");
    assert_eq!(result[0].email, "bob@example.com");
    assert_eq!(result[0].commit_count, 2);
    assert_eq!(result[1].name, "Alice", "Alice should be second (1 commit)");
    assert_eq!(result[1].commit_count, 1);
}

#[test]
fn test_enumerate_authors_same_name_different_emails_separate_entries() {
    let (_dir, repo) = common::create_fixture_repo(); // Alice <alice@example.com>: 1 commit
    common::add_commit_with_message(&repo, "Alice", "alice@old.com", "Alice old email commit");
    common::add_commit_with_message(&repo, "Alice", "alice@new.com", "Alice new email commit");

    let result = enumerate_authors(&repo).unwrap();
    assert_eq!(
        result.len(),
        3,
        "Expected three entries: fixture Alice + alice@old.com + alice@new.com"
    );
    // All have count 1 — deduplicated by exact (name, email) pair
    for entry in &result {
        assert_eq!(entry.commit_count, 1, "Each distinct (name, email) should have count 1");
    }
    // Verify the three distinct emails are all present
    let emails: Vec<&str> = result.iter().map(|e| e.email.as_str()).collect();
    assert!(emails.contains(&"alice@example.com"), "Should contain fixture alice@example.com");
    assert!(emails.contains(&"alice@old.com"), "Should contain alice@old.com");
    assert!(emails.contains(&"alice@new.com"), "Should contain alice@new.com");
}

#[test]
fn test_enumerate_coauthors_case_insensitive_dedup() {
    let (_dir, repo) = common::create_fixture_repo();
    // Three case variants of Co-authored-by — all same Name+Email, should deduplicate to count=3
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: first\n\nCo-authored-by: Charlie <charlie@x.com>",
    );
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: second\n\nCO-AUTHORED-BY: Charlie <charlie@x.com>",
    );
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: third\n\nco-authored-by: Charlie <charlie@x.com>",
    );

    let result = enumerate_coauthors(&repo).unwrap();
    assert_eq!(
        result.len(),
        1,
        "All three case variants should deduplicate to one entry"
    );
    assert_eq!(result[0].name, "Charlie");
    assert_eq!(result[0].email, "charlie@x.com");
    assert_eq!(result[0].commit_count, 3);
}

#[test]
fn test_enumerate_coauthors_returns_empty_when_none() {
    let (_dir, repo) = common::create_fixture_repo();
    // No co-author trailers in any commit

    let result = enumerate_coauthors(&repo).unwrap();
    assert_eq!(result, Vec::<CoAuthorEntry>::new());
}

#[test]
fn test_enumerate_coauthors_malformed_trailer_skipped() {
    let (_dir, repo) = common::create_fixture_repo();
    // Malformed: no angle brackets — should be silently ignored
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: malformed\n\nCo-authored-by: just a name no brackets",
    );
    // Valid: should be counted
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: valid\n\nCo-authored-by: Dave <dave@x.com>",
    );

    let result = enumerate_coauthors(&repo).unwrap();
    assert_eq!(result.len(), 1, "Only Dave should appear; malformed line is silently ignored");
    assert_eq!(result[0].name, "Dave");
    assert_eq!(result[0].email, "dave@x.com");
    assert_eq!(result[0].commit_count, 1);
}

#[test]
fn test_enumerate_coauthors_two_distinct_sorted_desc() {
    let (_dir, repo) = common::create_fixture_repo();
    // Eve appears twice, Frank once — Eve should be first (sorted desc)
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: first\n\nCo-authored-by: Eve <eve@x.com>",
    );
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: second\n\nCo-authored-by: Eve <eve@x.com>",
    );
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: third\n\nCo-authored-by: Frank <frank@x.com>",
    );

    let result = enumerate_coauthors(&repo).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Eve", "Eve should be first (2 commits)");
    assert_eq!(result[0].commit_count, 2);
    assert_eq!(result[1].name, "Frank", "Frank should be second (1 commit)");
    assert_eq!(result[1].commit_count, 1);
}
