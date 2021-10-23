use git2::build::CheckoutBuilder;
use git2::{Cred, RemoteCallbacks, Repository};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use crate::{Config, ConfigOverrides, Dependency, DependencyOverride};

#[derive(Debug)]
pub struct Resolver {
    config: Config,
    config_overrides: Option<ConfigOverrides>,
}

impl Resolver {
    pub fn new(config: Config) -> Self {
        Resolver {
            config,
            config_overrides: None,
        }
    }

    pub fn with_config_overrides(mut self, config_overrides: ConfigOverrides) -> Self {
        self.config_overrides = Some(config_overrides);

        self
    }

    pub fn resolve_all(&self) -> HashMap<String, ResolvedDependency> {
        let overrides = self.config_overrides.as_ref().map(|c| &c.dependencies);
        let map: HashMap<String, ResolvedDependency> = self
            .config
            .dependencies
            .iter()
            .map(|(key, value)| {
                let value = match overrides.and_then(|o| o.get(key)) {
                    None => match value {
                        Dependency::GitRepository { git_repo, git_ref } => {
                            ResolvedDependency::GitRepository {
                                url: git_repo.clone(),
                                fetch_ref: git_ref.to_fetch_ref(),
                                checkout_ref: git_ref.to_checkout_refspec(),
                            }
                        }
                    },
                    Some(o) => match o {
                        DependencyOverride::GitRepository { git_repo, git_ref } => {
                            ResolvedDependency::GitRepository {
                                url: git_repo
                                    .as_ref()
                                    .or(value.git_repo_url())
                                    .expect("TODO err handling")
                                    .clone(),
                                fetch_ref: git_ref.to_fetch_ref(),
                                checkout_ref: git_ref.to_checkout_refspec(),
                            }
                        }
                        DependencyOverride::LocalPath { local_path } => {
                            ResolvedDependency::LocalPath {
                                local_path: local_path.clone(),
                            }
                        }
                    },
                };

                (key.clone(), value)
            })
            .collect();

        map
    }
}

#[derive(Debug)]
pub enum ResolvedDependency {
    GitRepository {
        url: String,
        fetch_ref: String,
        checkout_ref: String,
    },
    LocalPath {
        local_path: PathBuf,
    },
}

fn fetch_opts() -> git2::FetchOptions<'static> {
    // Prepare callbacks.
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });

    // Prepare fetch options.
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    fo
}

fn clone_repo(url: &str, target_dir: &Path) -> Result<Repository, git2::Error> {
    let fo = fetch_opts();

    // Prepare builder.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    // Clone the project.
    builder.clone(url, target_dir)
}

impl ResolvedDependency {
    pub fn acquire(&self, target_dir: &Path) {
        match self {
            ResolvedDependency::GitRepository {
                url,
                fetch_ref,
                checkout_ref,
            } => {
                if std::fs::symlink_metadata(target_dir).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                    symlink::remove_symlink_dir(target_dir).expect("could not remove symlink");
                }

                let repo = if target_dir.exists() {
                    Repository::open(target_dir).expect("could not open")
                } else {
                    println!("  cloning into {}...", target_dir.display());
                    clone_repo(url, target_dir).expect("could not clone")
                };

                let head_ref = repo.head().expect("could not get HEAD").resolve().unwrap();
                let latest_commit = head_ref.peel_to_commit().unwrap();
                let prev_latest_commit = latest_commit.id();

                repo.find_remote("origin")
                    .unwrap()
                    .fetch(&[fetch_ref], Some(&mut fetch_opts()), None)
                    .expect("failed to fetch");
                repo.set_head(&checkout_ref).expect("invalid ref");
                repo.checkout_head(Some(CheckoutBuilder::new().force()))
                    .expect("could not reset");
                let head_ref = repo.head().expect("could not get HEAD");
                let latest_commit = head_ref.peel_to_commit().unwrap();

                if prev_latest_commit == latest_commit.id() {
                    println!("  at commit {:?}", prev_latest_commit);
                } else {
                    println!(
                        "  updated to commit {:?} (from {:?})",
                        latest_commit.id(),
                        prev_latest_commit
                    );
                }
            }
            ResolvedDependency::LocalPath { local_path } => {
                symlink::symlink_dir(local_path, target_dir).expect("failed to symlink");
            }
        }
    }
}
