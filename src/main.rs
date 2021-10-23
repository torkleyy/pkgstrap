use std::fs::read_to_string;
use std::path::PathBuf;

use pkgstrap::*;
use ron_reboot::from_str;

fn main() {
    let config: Config = from_str(&read_to_string("pkgstrap.ron").unwrap()).unwrap();

    let resolver = Resolver::new(config);

    let resolved = resolver.resolve_all();

    println!("{:#?}", resolved);

    let pkgstrap_base = PathBuf::from(".pkgstrap");
    std::fs::create_dir_all(&pkgstrap_base).unwrap();

    for (name, dep) in resolved.iter() {
        println!("Setting up dependency {}...", name);

        dep.acquire(&pkgstrap_base.join(name));
    }
}
