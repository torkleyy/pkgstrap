use std::{collections::HashMap, path::PathBuf};

use git2::Reference;
use serde::Deserialize;

mod resolved;

pub use anyhow::{Error, Result};

pub use self::resolved::{DependencyDirs, ResolvedDependency, Resolver};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Dependency {
    pub source: DependencySource,
    pub target: Option<PathBuf>,
}

impl Dependency {}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencySource {
    GitRepository {
        git_repo: String,
        #[serde(flatten)]
        git_ref: GitRef,
    },
}

impl DependencySource {
    pub fn git_repo_url(&self) -> Option<&String> {
        match self {
            DependencySource::GitRepository { git_repo, .. } => Some(git_repo),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ConfigOverrides {
    pub dependencies: HashMap<String, DependencyOverride>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DependencyOverride {
    LocalPath {
        local_path: PathBuf,
    },
    GitRepository {
        git_repo: Option<String>,
        #[serde(flatten)]
        git_ref: GitRef,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Directories {
    pub pkgstrap_dir: PathBuf,
    pub deps_dir: PathBuf,
    pub local_git_workdirs: PathBuf,
    pub global_git_repos: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
