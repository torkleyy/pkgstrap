use std::fs::create_dir_all;
use std::str::FromStr;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use git2::{build::CheckoutBuilder, Cred, RemoteCallbacks, Repository};
use url::Url;

use crate::{Config, ConfigOverrides, DependencyOverride, DependencySource, Result};

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

    pub fn resolve_all(&self) -> Result<HashMap<String, ResolvedDependency>> {
        let overrides = self.config_overrides.as_ref().map(|c| &c.dependencies);
        let map: Result<HashMap<String, ResolvedDependency>> = self
            .config
            .dependencies
            .iter()
            .map(|(key, value)| {
                let value = match overrides.and_then(|o| o.get(key)) {
                    None => match &value.source {
                        DependencySource::GitRepository { git_repo, git_ref } => {
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
                                    .or(value.source.git_repo_url())
                                    .ok_or_else(|| anyhow!("override for {} specifies git ref without repo url but root config does not provide repo url either", key))?
                                    .clone(),
                                fetch_ref: git_ref.to_fetch_ref(),
                                checkout_ref: git_ref.to_checkout_refspec(),
                            }
                        }
                        DependencyOverride::LocalPath { local_path } => {
                            ResolvedDependency::LocalPath {
                                local_path: {
                                    local_path.canonicalize().with_context(|| anyhow!("path {} invalid or not supported", local_path.display()))?;

                                    // preserve user's path spec
                                    local_path.clone()
                                },
                            }
                        }
                    },
                };

                Ok((key.clone(), value))
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
            username_from_url.unwrap_or("git"),
            None,
            &dirs::home_dir()
                .unwrap_or(".".into())
                .join(".ssh")
                .join("id_rsa"),
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

fn normalize_url_for_dir(url: &str) -> Result<PathBuf> {
    let url = Url::from_str(url).context("could not parse url")?;
    let domain = url.domain().context("missing domain")?;
    let mut path: PathBuf = domain.into();
    for segment in url.path_segments().into_iter().flatten() {
        path.push(segment);
    }
    path.set_extension("");

    Ok(path)
}

fn safe_symlink_dir(symlink_dir: &Path, existing_dir: &Path) -> Result<()> {
    if std::fs::symlink_metadata(symlink_dir)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        symlink::remove_symlink_dir(symlink_dir)
            .with_context(|| anyhow!("could not remove symlink {}", symlink_dir.display()))?;
    } else if symlink_dir.exists() {
        bail!(
            "dir {} already exists but is not a symlink",
            symlink_dir.display()
        )
    }

    let existing_dir = existing_dir
        .canonicalize()
        .with_context(|| anyhow!("path {} invalid or unsupported", existing_dir.display()))?;

    symlink::symlink_dir(existing_dir, symlink_dir).context("failed to symlink")
}

impl ResolvedDependency {
    pub fn acquire(&self, git_base_dir: &Path, target_dir: &Path) -> Result<()> {
        match self {
            ResolvedDependency::GitRepository {
                url,
                fetch_ref,
                checkout_ref,
            } => {
                let git_dir = git_base_dir.join(normalize_url_for_dir(url)?);
                let git_dir = &git_dir;
                let parent_git_dir = git_dir.parent().unwrap();
                create_dir_all(parent_git_dir).with_context(|| {
                    anyhow!(
                        "failed to create git parent dir {}",
                        parent_git_dir.display()
                    )
                })?;

                let repo = if git_dir.exists() {
                    Repository::open(git_dir).context("could not open repo")?
                } else {
                    println!("  cloning into {}...", git_dir.display());
                    clone_repo(url, git_dir).context("could not clone repo")?
                };

                let head_ref = repo.head().expect("could not get HEAD").resolve().unwrap();
                let latest_commit = head_ref.peel_to_commit().unwrap();
                let prev_latest_commit = latest_commit.id();

                repo.remote_anonymous(url)
                    .context("invalid remote")?
                    .fetch(&[fetch_ref], Some(&mut fetch_opts()), None)
                    .with_context(|| anyhow!("failed to fetch from {}", url))?;
                repo.set_head(&checkout_ref)
                    .context("cannot switch to ref")?;
                repo.checkout_head(Some(CheckoutBuilder::new().force()))
                    .context("could not checkout HEAD")?;
                let head_ref = repo
                    .head()
                    .context("unexpected error while resolving HEAD")?;
                let latest_commit = head_ref
                    .peel_to_commit()
                    .context("unexpected error while resolving HEAD")?;

                safe_symlink_dir(target_dir, git_dir)?;

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
                safe_symlink_dir(target_dir, local_path)?;

                println!("  linked to {}", local_path.display());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::resolved::normalize_url_for_dir;

    #[test]
    fn normalize_urls() {
        assert_eq!(
            normalize_url_for_dir("https://github.com/torkleyy/async-rust-parser.git")
                .unwrap()
                .display()
                .to_string()
                .replace("\\", "/"),
            "github.com/torkleyy/async-rust-parser"
        );
    }
}
