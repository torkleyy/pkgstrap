use std::{
    fs,
    fs::{read_to_string, rename},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use pkgstrap_lib::*;
use remove_dir_all::remove_dir_all;
use ron_reboot::from_str;
use structopt::StructOpt;

/// pkgstrap
///
/// Sets up dependencies, especially Git repositories,
/// creates symlinks and allows overrides as needed.
#[derive(StructOpt, Debug)]
#[structopt(name = "pkgstrap")]
struct Opt {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,
    #[structopt(long, default_value = "pkgstrap.ron")]
    config: PathBuf,
    #[structopt(long, default_value = ".pkgstrap")]
    pkgstrap_dir: PathBuf,
    #[structopt(subcommand)]
    subcommand: Option<SubCommand>,
}

#[derive(StructOpt, Debug)]
enum SubCommand {
    /// Cleans up dependency symlinks & git repos
    Clean {
        /// Whether to clean deps directory.
        #[structopt(long = "no-deps-dir", parse(from_flag = std::ops::Not::not))]
        deps_dir: bool,
        /// Whether to clean git repos.
        #[structopt(long = "no-git", parse(from_flag = std::ops::Not::not))]
        git: bool,
        /// Whether to "clean" overrides as well. Will rename the file to `overrides.ron.bk`.
        #[structopt(long)]
        overrides: bool,
    },
    /// Clone a dependency and setup an override
    Clone {
        dependency: String,
        target: PathBuf,
    }
}

trait OrPrint {
    fn or_print(self);
}

impl<T> OrPrint for Result<T> {
    fn or_print(self) {
        match self {
            Err(e) => print_err(e),
            _ => {}
        }
    }
}

fn print_err(e: Error) {
    eprintln!("error: {}", e);
    e.chain()
        .skip(1)
        .for_each(|cause| eprintln!("caused by: {}", cause));
}

fn main() {
    if let Err(e) = app() {
        print_err(e);
        std::process::exit(1);
    }
}

fn app() -> Result<()> {
    let matches: Opt = Opt::from_args();

    let directories = Directories {
        deps_dir: matches.pkgstrap_dir.join("deps"),
        local_git_workdirs: matches.pkgstrap_dir.join("git"),
        pkgstrap_dir: matches.pkgstrap_dir,
        global_git_repos: dirs::home_dir()
            .context("no home dir")?
            .join(".pkgstrap")
            .join("git-repos"),
    };

    let Directories {
        pkgstrap_dir,
        deps_dir,
        local_git_workdirs,
        global_git_repos,
    } = &directories;
    let override_file = pkgstrap_dir.join("overrides.ron");
    let override_file = &override_file;
    let config_file = &matches.config;

    match matches.subcommand {
        None => {
            let config_contents = read_to_string(config_file).context("could not open config")?;

            let config: Config = from_str(&config_contents).context("could not parse config")?;

            let mut resolver = Resolver::new(config.clone());

            std::fs::create_dir_all(&pkgstrap_dir).unwrap();
            std::fs::create_dir_all(&deps_dir).unwrap();
            std::fs::create_dir_all(&global_git_repos).unwrap();
            std::fs::create_dir_all(&local_git_workdirs).unwrap();

            if override_file.exists() {
                let overrides: ConfigOverrides =
                    from_str(&read_to_string(&override_file).context("could not open overrides")?)
                        .context("could not parse overrides")?;
                resolver = resolver.with_config_overrides(overrides);
            }

            let resolved = resolver.resolve_all()?;

            for (name, dep) in resolved.iter() {
                println!("Setting up dependency {}...", name);

                let target = config.dependencies[name]
                    .target
                    .clone()
                    .unwrap_or_else(|| deps_dir.join(name));
                dep.acquire(DependencyDirs {
                    base: &directories,
                    std_target_dir: &target,
                    in_tree_target_dirs: vec![],

                    local_git_worktree: &local_git_workdirs.join(name),
                })
                .with_context(|| anyhow!("failed to acquire dependency {}", name))?;
            }

            fs::write(pkgstrap_dir.join("pkgstrap.ron.last"), config_contents)
                .context("could not backup config")?;
        }
        Some(SubCommand::Clean {
            deps_dir: clean_deps_dir,
            git: clean_git_dir,
            overrides: clean_overrides,
        }) => {
            println!("note: the clean subcommand does not work reliably and may print errors");

            if clean_deps_dir && deps_dir.exists() {
                remove_dir_all(deps_dir)
                    .context("could not remove deps dir")
                    .or_print();
            }
            if clean_git_dir && local_git_workdirs.exists() {
                // TODO: remove workdir from parent repo
                remove_dir_all(local_git_workdirs)
                    .context("could not remove git dir")
                    .or_print();
            }

            if clean_overrides && override_file.exists() {
                let mut to = override_file.clone();
                to.set_file_name("overrides");
                to.set_extension("ron.bk");
                rename(override_file, to)
                    .context("failed to rename overrides")
                    .or_print();
            }
        }
        Some(SubCommand::Clone { dependency: _, target: _ }) => todo!()
    }

    Ok(())
}
