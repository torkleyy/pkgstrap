use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use git2::{Branch, BranchType, ObjectType, Oid, ResetType};

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
                        Dependency::GitRepository {
                            git_repo,
                            branch,
                            git_ref,
                        } => ResolvedDependency::GitRepository {
                            url: git_repo.clone(),
                            branch: branch.clone(),
                            git_ref: git_ref.clone(),
                        },
                    },
                    Some(o) => match o {
                        DependencyOverride::GitRepository {
                            git_repo,
                            branch,
                            git_ref,
                        } => ResolvedDependency::GitRepository {
                            url: git_repo
                                .as_ref()
                                .or(value.git_repo_url())
                                .expect("TODO err handling")
                                .clone(),
                            branch: branch.as_ref().or(value.git_branch()).cloned(),
                            git_ref: git_ref.as_ref().or(value.git_ref()).cloned(),
                        },
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
        branch: Option<String>,
        git_ref: Option<String>,
    },
    LocalPath {
        local_path: PathBuf,
    },
}

impl ResolvedDependency {
    pub fn acquire(&self, target_dir: &Path) {
        use git2::{Cred, ErrorCode, Repository, RemoteCallbacks};

        match self {
            ResolvedDependency::GitRepository { url, branch, git_ref } => {
                let branch_name = branch.as_ref().map(|s| s.as_str()).unwrap_or("main");
                let git_ref = git_ref.clone().unwrap_or_else(|| format!("refs/remotes/origin/{}", branch_name));
                //assert!(git2::Reference::is_valid_name(&git_ref));

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

                // Prepare builder.
                let mut builder = git2::build::RepoBuilder::new();
                builder.fetch_options(fo);

                // Clone the project.
                let repo = match builder.clone(
                    url, target_dir
                ) {
                    Err(e) if e.code() == ErrorCode::Exists => {
                        Repository::open(target_dir).expect("could not open")
                    }
                    Err(e) => {
                        eprintln!("clone error: {}", e);
                        todo!()
                    }
                    Ok(repo) => {
                        repo
                    }
                };

                let head_ref = repo.head().expect("could not get HEAD").resolve().unwrap();
                let latest_commit = head_ref.peel_to_commit().unwrap();
                let prev_latest_commit = latest_commit.id();

                repo.find_remote("origin").unwrap().fetch(&[branch_name], None, None).expect("failed to fetch");
                let ref_object = repo.revparse(&git_ref).expect("invalid ref").from().expect("ref has no from").clone();
                repo.checkout_head(None).expect("checkout failed");
                repo.reset(&ref_object, ResetType::Hard, None).expect("failed to reset repo");
                let head_ref = repo.head().expect("could not get HEAD").resolve().unwrap();
                let latest_commit = head_ref.peel_to_commit().unwrap();

                if prev_latest_commit == latest_commit.id() {
                    println!("up to date");
                } else {
                    println!("updated to commit {:?}", latest_commit.id());
                }
            }
            ResolvedDependency::LocalPath { local_path } => {
                symlink::symlink_dir(local_path, target_dir).expect("failed to symlink");
            }
        }
    }
}
