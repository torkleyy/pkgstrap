use git2::Reference;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

mod resolved;

pub use self::resolved::{ResolvedDependency, Resolver};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    GitRepository {
        git_repo: String,
        #[serde(flatten)]
        git_ref: GitRef,
    },
}

impl Dependency {
    pub fn git_repo_url(&self) -> Option<&String> {
        match self {
            Dependency::GitRepository { git_repo, .. } => Some(git_repo),
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
        #[serde(flatten)]
        git_ref: GitRef,
    },
    LocalPath {
        local_path: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GitRef {
    Branch { branch: String },
    Tag { tag: String },
    Commit { branch: String, commit: String },
}

impl GitRef {
    pub fn to_fetch_ref(&self) -> String {
        let r = match self {
            GitRef::Branch { branch } => branch.clone(),
            GitRef::Tag { tag } => tag.clone(),
            GitRef::Commit { branch, .. } => branch.clone(),
        };

        r
    }

    pub fn to_checkout_refspec(&self) -> String {
        let r = match self {
            GitRef::Branch { branch } => {
                format!("refs/remotes/origin/{}", branch)
            }
            GitRef::Tag { tag } => {
                format!("refs/tags/{}", tag)
            }
            GitRef::Commit { commit, .. } => {
                // Skip assertion
                return commit.clone();
            }
        };

        assert!(Reference::is_valid_name(&r));

        r
    }
}

#[cfg(test)]
mod tests {
    use crate::GitRef;

    #[test]
    fn checkout_refs() {
        assert_eq!(
            GitRef::Branch {
                branch: "main".to_string()
            }
            .to_checkout_refspec(),
            "refs/remotes/origin/main"
        );
        assert_eq!(
            GitRef::Tag {
                tag: "1.0.0".to_string()
            }
            .to_checkout_refspec(),
            "refs/tags/1.0.0"
        );
        assert_eq!(
            GitRef::Commit {
                branch: "main".to_string(),
                commit: "12f123".to_owned()
            }
            .to_checkout_refspec(),
            "12f123"
        );
    }
}
