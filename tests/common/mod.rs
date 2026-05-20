#![allow(dead_code)]

use git2::{Repository, Signature, Time};
use tempfile::TempDir;

pub fn create_fixture_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let sig = Signature::new("Alice", "alice@example.com", &Time::new(1_000_000, 0)).unwrap();
    let tree_oid = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    (dir, repo)
}

pub fn add_commit_with_message(repo: &Repository, name: &str, email: &str, message: &str) {
    let sig = Signature::new(name, email, &git2::Time::new(1_000_001, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}

pub fn create_branch(repo: &Repository, name: &str, target: &git2::Commit) {
    repo.branch(name, target, false).unwrap();
}

pub fn add_merge_commit(
    repo: &Repository,
    name: &str,
    email: &str,
    message: &str,
    parent0: &git2::Commit,
    parent1: &git2::Commit,
) {
    let sig = Signature::new(name, email, &git2::Time::new(1_000_002, 0)).unwrap();
    let tree = parent0.tree().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &[parent0, parent1],
    )
    .unwrap();
}

pub fn create_annotated_tag(repo: &Repository, name: &str, target: &git2::Commit, message: &str) {
    let tagger = Signature::new(
        "Tagger",
        "tagger@example.com",
        &git2::Time::new(2_000_000, 0),
    )
    .unwrap();
    repo.tag(name, target.as_object(), &tagger, message, false)
        .unwrap();
}
