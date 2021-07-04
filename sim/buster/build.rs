use kaze::*;
use rtl::*;

use std::env;
use std::fs::File;
use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("modules.rs");
    let mut file = File::create(&dest_path).unwrap();

    let c = Context::new();

    sim::generate(BusterCrossbar::new("buster_1x2", 1, 2, 17, 1, 32, 2, &c).m, sim::GenerationOptions {
        override_module_name: Some("Buster1x2".into()),
        ..sim::GenerationOptions::default()
    }, &mut file)?;
    sim::generate(BusterCrossbar::new("buster_2x1", 2, 1, 16, 0, 32, 2, &c).m, sim::GenerationOptions {
        override_module_name: Some("Buster2x1".into()),
        ..sim::GenerationOptions::default()
    }, &mut file)?;
    sim::generate(BusterCrossbar::new("buster_2x2", 2, 2, 17, 1, 128, 4, &c).m, sim::GenerationOptions {
        override_module_name: Some("Buster2x2".into()),
        ..sim::GenerationOptions::default()
    }, &mut file)?;

    Ok(())
}
