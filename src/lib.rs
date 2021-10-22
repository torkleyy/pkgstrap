use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

mod resolved;

pub use self::resolved::{Resolver, ResolvedDependency};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    GitRepository {
        git_repo: String,
        branch: Option<String>,
        #[serde(rename = "ref")]
        git_ref: Option<String>,
    },
}

impl Dependency {
    pub fn git_repo_url(&self) -> Option<&String> {
        match self {
            Dependency::GitRepository { git_repo, .. } => Some(git_repo),
        }
    }

    pub fn git_branch(&self) -> Option<&String> {
        match self {
            Dependency::GitRepository { git_ref, .. } => git_ref.as_ref(),
        }
    }

    pub fn git_ref(&self) -> Option<&String> {
        match self {
            Dependency::GitRepository { branch, .. } => branch.as_ref(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigOverrides {
    pub dependencies: HashMap<String, DependencyOverride>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencyOverride {
    GitRepository {
        git_repo: Option<String>,
        branch: Option<String>,
        #[serde(rename = "ref")]
        git_ref: Option<String>,
    },
    LocalPath {
        local_path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
