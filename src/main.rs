use std::{fs, fs::read_to_string, path::PathBuf};

use anyhow::{anyhow, Context};
use pkgstrap::*;
use ron_reboot::from_str;
use structopt::StructOpt;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "pkgstrap")]
struct Opt {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,
}

fn main() {
    if let Err(e) = app() {
        eprintln!("error: {}", e);
        e.chain()
            .skip(1)
            .for_each(|cause| eprintln!("caused by: {}", cause));
        std::process::exit(1);
    }
}

fn app() -> Result<()> {
    let _matches = Opt::from_args();

    let config_file = "pkgstrap.ron";
    let config_contents = read_to_string(config_file).context("could not open config")?;

    let config: Config = from_str(&config_contents).context("could not parse config")?;

    let mut resolver = Resolver::new(config.clone());

    let pkgstrap_base = PathBuf::from(".pkgstrap");
    std::fs::create_dir_all(&pkgstrap_base).unwrap();
    let deps_base = pkgstrap_base.join("deps");
    std::fs::create_dir_all(&deps_base).unwrap();
    let git_base = pkgstrap_base.join("git");
    std::fs::create_dir_all(&deps_base).unwrap();

    let override_file = pkgstrap_base.join("overrides.ron");
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

    Ok(())
}
