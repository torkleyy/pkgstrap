use std::{fs::read_to_string, path::PathBuf};

use anyhow::{anyhow, Context};
use pkgstrap::*;
use ron_reboot::from_str;

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
    let config: Config =
        from_str(&read_to_string("pkgstrap.ron").context("could not open config")?)
            .context("could not parse config")?;

    let resolver = Resolver::new(config.clone());

    let resolved = resolver.resolve_all()?;

    println!("{:#?}", resolved);

    let pkgstrap_base = PathBuf::from(".pkgstrap");
    std::fs::create_dir_all(&pkgstrap_base).unwrap();

    for (name, dep) in resolved.iter() {
        println!("Setting up dependency {}...", name);

        let target = config.dependencies[name]
            .target
            .clone()
            .unwrap_or_else(|| pkgstrap_base.join(name));
        dep.acquire(&target)
            .with_context(|| anyhow!("failed to acquire dependency {}", name))?;
    }

    Ok(())
}
