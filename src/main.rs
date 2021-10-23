use std::fs::{ rename};
use std::{fs, fs::read_to_string, path::PathBuf};

use anyhow::{anyhow, Context};
use pkgstrap::*;
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

    let pkgstrap_base = matches.pkgstrap_dir;
    let pkgstrap_base = &pkgstrap_base;
    let override_file = pkgstrap_base.join("overrides.ron");
    let override_file = &override_file;
    let config_file = &matches.config;
    let deps_base = pkgstrap_base.join("deps");
    let deps_base = &deps_base;
    let git_base = pkgstrap_base.join("git");
    let git_base = &git_base;

    match matches.subcommand {
        None => {
            let config_contents = read_to_string(config_file).context("could not open config")?;

            let config: Config = from_str(&config_contents).context("could not parse config")?;

            let mut resolver = Resolver::new(config.clone());

            std::fs::create_dir_all(&pkgstrap_base).unwrap();
            std::fs::create_dir_all(&deps_base).unwrap();
            std::fs::create_dir_all(&git_base).unwrap();

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
                    .unwrap_or_else(|| deps_base.join(name));
                dep.acquire(&git_base, &target)
                    .with_context(|| anyhow!("failed to acquire dependency {}", name))?;
            }

            fs::write(pkgstrap_base.join("pkgstrap.ron.last"), config_contents)
                .context("could not backup config")?;
        }
        Some(SubCommand::Clean {
            deps_dir,
            git,
            overrides,
        }) => {
            if deps_dir && deps_base.exists() {
                remove_dir_all(deps_base)
                    .context("could not remove deps dir")
                    .or_print();
            }
            if git && git_base.exists() {
                remove_dir_all(git_base)
                    .context("could not remove git dir")
                    .or_print();
            }

            if overrides && override_file.exists() {
                let mut to = override_file.clone();
                to.set_file_name("overrides");
                to.set_extension("ron.bk");
                rename(override_file, to)
                    .context("failed to rename overrides")
                    .or_print();
            }
        }
    }

    Ok(())
}
