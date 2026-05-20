mod common;

use git2::{ObjectType, Repository, Signature};
use git_author_reformer::git::rewrite::drop_coauthor;
use git_author_reformer::git::rewrite::rewrite_author;

fn find_commit_by_message<'a>(repo: &'a Repository, target_message: &str) -> git2::Commit<'a> {
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_glob("refs/heads/*").unwrap();
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();
    for oid_result in revwalk {
        let oid = oid_result.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        if commit.message_raw().unwrap_or("") == target_message {
            return commit;
        }
    }
    panic!(
        "find_commit_by_message: no commit with message {:?}",
        target_message
    );
}

#[test]
fn test_rewrite_author_removes_old_identity_across_all_branches() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice second");
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    common::create_branch(&repo, "feature", &head_commit);

    let count = rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    assert!(
        count >= 2,
        "rewrite_author must rewrite at least 2 commits (RENAME-03); got: {count}"
    );

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_glob("refs/heads/*").unwrap();
    let mut old_identity_count = 0usize;
    for oid_result in revwalk {
        let oid = oid_result.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let author = commit.author();
        if author.name().unwrap_or("") == "Alice"
            && author.email().unwrap_or("") == "alice@example.com"
        {
            old_identity_count += 1;
        }
    }
    assert_eq!(
        old_identity_count,
        0,
        "after rename, zero commits should retain old Alice identity across all branches (RENAME-03); found {old_identity_count} remaining"
    );
}

#[test]
fn test_rewrite_author_preserves_merge_parent_order() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice-main");
    let main_tip = repo.head().unwrap().peel_to_commit().unwrap();
    common::create_branch(&repo, "feature", &main_tip);
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice-main-2");
    let current_main_tip = repo.head().unwrap().peel_to_commit().unwrap();

    let parent0_msg = current_main_tip.message_raw().unwrap_or("").to_string();
    let parent1_msg = main_tip.message_raw().unwrap_or("").to_string();

    common::add_merge_commit(
        &repo,
        "Alice",
        "alice@example.com",
        "merge-msg",
        &current_main_tip,
        &main_tip,
    );

    rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    let new_merge = find_commit_by_message(&repo, "merge-msg");
    assert_eq!(
        new_merge.parent_count(),
        2,
        "merge commit must retain exactly 2 parents after rewrite (Phase 2 success criterion 3)"
    );

    let new_p0 = repo.find_commit(new_merge.parent_id(0).unwrap()).unwrap();
    let new_p1 = repo.find_commit(new_merge.parent_id(1).unwrap()).unwrap();

    assert_eq!(
        new_p0.message_raw().unwrap_or(""),
        parent0_msg.as_str(),
        "merge first parent must remain first after rewrite (Phase 2 success criterion 3); got: {:?}",
        new_p0.message_raw()
    );
    assert_eq!(
        new_p1.message_raw().unwrap_or(""),
        parent1_msg.as_str(),
        "merge second parent must remain second after rewrite (Phase 2 success criterion 3); got: {:?}",
        new_p1.message_raw()
    );
}

#[test]
fn test_rewrite_author_recreates_annotated_tag_object() {
    let (_dir, repo) = common::create_fixture_repo();
    let alice_commit = repo.head().unwrap().peel_to_commit().unwrap();
    common::create_annotated_tag(&repo, "v1", &alice_commit, "release v1");

    let tag_ref = repo.find_reference("refs/tags/v1").unwrap();
    let tag_obj_oid = tag_ref.target().unwrap();
    let tag_obj = repo.find_object(tag_obj_oid, None).unwrap();
    assert_eq!(
        tag_obj.kind(),
        Some(ObjectType::Tag),
        "refs/tags/v1 must start as an annotated tag object (pre-condition for RENAME-04)"
    );
    let old_target = tag_obj.as_tag().unwrap().target_id();

    rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    let tag_ref_after = repo.find_reference("refs/tags/v1").unwrap();
    let new_tag_obj_oid = tag_ref_after.target().unwrap();
    let new_tag_obj = repo.find_object(new_tag_obj_oid, None).unwrap();
    assert_eq!(
        new_tag_obj.kind(),
        Some(ObjectType::Tag),
        "after rewrite, refs/tags/v1 must still point at an annotated tag object (RENAME-04 requires tag OBJECT recreation, not just ref update)"
    );

    let new_target = new_tag_obj.as_tag().unwrap().target_id();
    assert_ne!(
        new_target,
        old_target,
        "after rewrite, annotated tag's target commit OID must differ (RENAME-04): tag must point at the new commit, not the old one"
    );

    let new_target_commit = repo.find_commit(new_target).unwrap();
    assert_eq!(
        new_target_commit.author().name().unwrap_or(""),
        "Alice Renamed",
        "the commit pointed to by the recreated annotated tag must have the new author name (RENAME-04)"
    );
}

#[test]
fn test_rewrite_author_only_rewrites_committer_when_committer_matches_old_author() {
    let (_dir, repo) = common::create_fixture_repo();

    let alice_sig =
        Signature::new("Alice", "alice@example.com", &git2::Time::new(1_000_010, 0)).unwrap();
    let bob_sig = Signature::new("Bob", "bob@example.com", &git2::Time::new(1_000_011, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(
        Some("HEAD"),
        &alice_sig,
        &bob_sig,
        "split-identity",
        &tree,
        &[&head],
    )
    .unwrap();

    rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    let rewritten = find_commit_by_message(&repo, "split-identity");

    assert_eq!(
        rewritten.author().name().unwrap_or(""),
        "Alice Renamed",
        "author MUST be rewritten when author matches old identity (RENAME-03)"
    );
    assert_eq!(
        rewritten.author().email().unwrap_or(""),
        "alice2@example.com",
        "author email MUST be rewritten when author matches old identity (RENAME-03)"
    );
    assert_eq!(
        rewritten.committer().name().unwrap_or(""),
        "Bob",
        "committer must NOT be rewritten when committer does not match old author identity (RENAME-03 'when the committer matches' clause); got: {:?}",
        rewritten.committer().name()
    );
    assert_eq!(
        rewritten.committer().email().unwrap_or(""),
        "bob@example.com",
        "committer email must NOT be rewritten when committer does not match old author identity (RENAME-03 'when the committer matches' clause); got: {:?}",
        rewritten.committer().email()
    );
}

#[test]
fn test_rewrite_author_updates_detached_head() {
    let (_dir, repo) = common::create_fixture_repo();
    let alice_commit_id = repo.head().unwrap().target().unwrap();
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice-second");
    repo.set_head_detached(alice_commit_id).unwrap();

    rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    assert!(
        repo.head_detached().unwrap(),
        "HEAD must remain detached after rewrite (Pitfall 4 handling)"
    );

    let new_head_oid = repo.head().unwrap().target().unwrap();
    assert_ne!(
        new_head_oid,
        alice_commit_id,
        "detached HEAD OID must be updated to the rewritten commit (Pitfall 4); HEAD still points at the old OID"
    );

    let new_head_commit = repo.find_commit(new_head_oid).unwrap();
    assert_eq!(
        new_head_commit.author().name().unwrap_or(""),
        "Alice Renamed",
        "detached HEAD commit must have the new author name after rewrite (Pitfall 4)"
    );
}

#[test]
fn test_rewrite_author_preserves_timestamps_and_message_byte_for_byte() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(&repo, "Bob", "bob@example.com", "bob-untouched");
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice-second");

    let bob_pre = find_commit_by_message(&repo, "bob-untouched");
    let captured_message = bob_pre.message_raw().unwrap_or("").to_string();
    let captured_seconds = bob_pre.author().when().seconds();
    let captured_offset = bob_pre.author().when().offset_minutes();
    let captured_tree_id = bob_pre.tree_id();
    drop(bob_pre);

    rewrite_author(
        &repo,
        "Alice",
        "alice@example.com",
        "Alice Renamed",
        "alice2@example.com",
    )
    .unwrap();

    let new_bob = find_commit_by_message(&repo, "bob-untouched");

    assert_eq!(
        new_bob.message_raw().unwrap_or(""),
        captured_message.as_str(),
        "Bob's commit message must be byte-identical after rewrite (Phase 2 success criterion 4 / DROP-03)"
    );
    assert_eq!(
        new_bob.author().name().unwrap_or(""),
        "Bob",
        "Bob's author name must be untouched by Alice rename (RENAME-03: only matching identity rewritten)"
    );
    assert_eq!(
        new_bob.author().email().unwrap_or(""),
        "bob@example.com",
        "Bob's author email must be untouched by Alice rename (RENAME-03)"
    );
    assert_eq!(
        new_bob.author().when().seconds(),
        captured_seconds,
        "Bob's author timestamp seconds must be preserved bit-exact after rewrite (Phase 2 success criterion 4)"
    );
    assert_eq!(
        new_bob.author().when().offset_minutes(),
        captured_offset,
        "Bob's author timestamp offset_minutes must be preserved bit-exact after rewrite (Phase 2 success criterion 4)"
    );
    assert_eq!(
        new_bob.tree_id(),
        captured_tree_id,
        "Bob's tree OID must be unchanged after rewrite (Phase 2 success criterion 4)"
    );
}

// --- Tests 7-12: drop_coauthor (plan 02-03) ---

fn find_commit_containing<'a>(repo: &'a Repository, substr: &str) -> git2::Commit<'a> {
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_glob("refs/heads/*").unwrap();
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();
    for oid_result in revwalk {
        let oid = oid_result.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        if commit.message_raw().unwrap_or("").contains(substr) {
            return commit;
        }
    }
    panic!(
        "find_commit_containing: no commit whose message contains {:?}",
        substr
    );
}

#[test]
fn test_drop_coauthor_removes_matching_trailer() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: x\n\nCo-authored-by: Bob <bob@example.com>\n",
    );

    let count = drop_coauthor(&repo, "bob@example.com").unwrap();
    assert!(
        count >= 1,
        "drop_coauthor must rewrite at least 1 commit (DROP-02); got: {count}"
    );

    let rewritten = find_commit_containing(&repo, "feat: x");
    let msg = rewritten.message_raw().unwrap_or("");
    assert!(
        !msg.to_ascii_lowercase().contains("co-authored-by:"),
        "DROP-02: rewritten commit must not contain any Co-authored-by line after drop; msg: {:?}",
        msg
    );
    assert!(
        msg.contains("feat: x"),
        "DROP-03: commit subject must be preserved after co-author drop; msg: {:?}",
        msg
    );
}

#[test]
fn test_drop_coauthor_case_insensitive_email_match() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: ci\n\nCo-Authored-By: Bob <BOB@EXAMPLE.COM>\n",
    );

    drop_coauthor(&repo, "bob@example.com").unwrap();

    let rewritten = find_commit_containing(&repo, "feat: ci");
    let msg = rewritten.message_raw().unwrap_or("");
    assert!(
        !msg.to_ascii_lowercase().contains("bob@example.com"),
        "DROP-02: case-insensitive key matching must remove BOB@EXAMPLE.COM when target is bob@example.com; msg: {:?}",
        msg
    );
}

#[test]
fn test_drop_coauthor_removes_all_occurrences_within_one_commit() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: pair\n\nCo-authored-by: Bob <bob@example.com>\nCo-authored-by: Bob <bob@example.com>\n",
    );

    drop_coauthor(&repo, "bob@example.com").unwrap();

    let rewritten = find_commit_containing(&repo, "feat: pair");
    let msg = rewritten.message_raw().unwrap_or("");
    let count = msg.to_ascii_lowercase().matches("co-authored-by:").count();
    assert_eq!(
        count,
        0,
        "DROP-02: removes all occurrences within a single commit if duplicated; found {count} remaining in: {:?}",
        msg
    );
}

#[test]
fn test_drop_coauthor_preserves_other_coauthors_in_same_commit() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: trio\n\nCo-authored-by: Bob <bob@example.com>\nCo-authored-by: Carol <carol@example.com>\nCo-authored-by: Dave <dave@example.com>\n",
    );

    drop_coauthor(&repo, "carol@example.com").unwrap();

    let rewritten = find_commit_containing(&repo, "feat: trio");
    let msg = rewritten.message_raw().unwrap_or("");
    assert!(
        msg.contains("bob@example.com"),
        "DROP-03: Bob must be preserved when only Carol is the drop target; msg: {:?}",
        msg
    );
    assert!(
        msg.contains("dave@example.com"),
        "DROP-03: Dave must be preserved when only Carol is the drop target; msg: {:?}",
        msg
    );
    assert!(
        !msg.contains("carol@example.com"),
        "DROP-02: Carol must be removed as the drop target; msg: {:?}",
        msg
    );
}

#[test]
fn test_drop_coauthor_preserves_body_trailers_tree_timestamps_author_committer() {
    let (_dir, repo) = common::create_fixture_repo();
    let raw_message = "feat: typo fix\n\nThis fixes the typo described in #123.\n\nSigned-off-by: Alice <alice@example.com>\nCo-authored-by: Bob <bob@example.com>\n";
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", raw_message);

    let pre_commit = find_commit_containing(&repo, "typo fix");
    let original_message = pre_commit.message_raw().unwrap_or("").to_string();
    let original_seconds = pre_commit.author().when().seconds();
    let original_offset = pre_commit.author().when().offset_minutes();
    let original_committer_seconds = pre_commit.committer().when().seconds();
    let original_tree_id = pre_commit.tree_id();
    drop(pre_commit);

    drop_coauthor(&repo, "bob@example.com").unwrap();

    let rewritten = find_commit_containing(&repo, "typo fix");
    let new_msg = rewritten.message_raw().unwrap_or("");

    let expected_message = original_message.replace("Co-authored-by: Bob <bob@example.com>\n", "");
    assert_eq!(
        new_msg,
        expected_message.as_str(),
        "DROP-03: rewritten message must equal original with only the Co-authored-by Bob line removed; got: {:?}",
        new_msg
    );
    assert_eq!(
        rewritten.author().name().unwrap_or(""),
        "Alice",
        "DROP-03: author name must be untouched"
    );
    assert_eq!(
        rewritten.author().email().unwrap_or(""),
        "alice@example.com",
        "DROP-03: author email must be untouched"
    );
    assert_eq!(
        rewritten.author().when().seconds(),
        original_seconds,
        "DROP-03: author timestamp seconds must be preserved bit-exact"
    );
    assert_eq!(
        rewritten.author().when().offset_minutes(),
        original_offset,
        "DROP-03: author timestamp offset_minutes must be preserved bit-exact"
    );
    assert_eq!(
        rewritten.committer().when().seconds(),
        original_committer_seconds,
        "DROP-03: committer timestamp seconds must be preserved bit-exact"
    );
    assert_eq!(
        rewritten.tree_id(),
        original_tree_id,
        "DROP-03: tree OID must be unchanged after co-author drop"
    );
    assert!(
        new_msg.contains("Signed-off-by: Alice"),
        "DROP-03: Signed-off-by trailer must be preserved; msg: {:?}",
        new_msg
    );
    assert!(
        new_msg.contains("This fixes the typo described in #123."),
        "DROP-03: commit body text must be preserved; msg: {:?}",
        new_msg
    );
}

#[test]
fn test_drop_coauthor_does_not_rewrite_commits_with_no_matching_trailer() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(
        &repo,
        "Alice",
        "alice@example.com",
        "feat: y\n\nCo-authored-by: Carol <carol@example.com>\n",
    );

    let old_tip_oid = repo.head().unwrap().target().unwrap();
    let count = drop_coauthor(&repo, "bob@example.com").unwrap();
    let new_tip_oid = repo.head().unwrap().target().unwrap();

    assert_eq!(
        count,
        0,
        "DROP-02: drop_coauthor must return 0 when no commit matches the target email; got: {count}"
    );
    assert_eq!(
        new_tip_oid,
        old_tip_oid,
        "DROP-02: branch tip must be unchanged when nothing was rewritten; old={old_tip_oid}, new={new_tip_oid}"
    );
}
