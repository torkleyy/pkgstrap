use std::collections::HashMap;
use std::path::PathBuf;

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
