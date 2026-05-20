#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorIdentity {
    pub name: String,
    pub email: String,
    pub commit_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoAuthorEntry {
    pub name: String,
    pub email: String,
    pub commit_count: usize,
}
