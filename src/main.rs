use std::fs::read_to_string;
use ron_reboot::from_str;
use pkgstrap::*;

fn main() {
    let config: Config = from_str(&read_to_string("pkgstrap.ron").unwrap()).unwrap();

    let resolver = Resolver::new(config);

    let resolved = resolver.resolve_all();

    println!("{:#?}", resolved);
}
