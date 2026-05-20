use git2::Repository;

pub struct App {
    pub repo: Repository,
    pub screen: Screen,
    pub should_exit: bool,
}

pub enum Screen {
    MainMenu { selected: usize },
    NotImplemented(&'static str),
}

pub enum MenuChoice {
    Rename,
    Drop,
}

impl MenuChoice {
    pub fn from_index(i: usize) -> Self {
        if i == 0 {
            Self::Rename
        } else {
            Self::Drop
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rename => "Rename an author",
            Self::Drop => "Drop a co-author",
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Rename, Self::Drop]
    }
}

impl App {
    pub fn new(repo: Repository) -> Self {
        Self {
            repo,
            screen: Screen::MainMenu { selected: 0 },
            should_exit: false,
        }
    }
}
