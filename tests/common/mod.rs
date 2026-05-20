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
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();
    drop(tree);

    (dir, repo)
}

pub fn add_commit_with_message(repo: &Repository, name: &str, email: &str, message: &str) {
    let sig = Signature::new(name, email, &git2::Time::new(1_000_001, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}
