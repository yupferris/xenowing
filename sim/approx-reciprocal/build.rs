use kaze::*;
use rtl::*;

use std::env;
use std::fs::File;
use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("modules.rs");
    let file = File::create(&dest_path).unwrap();

    let c = Context::new();

    const W_INVERSE_FRACT_BITS: u32 = 30;
    const RESTORED_W_FRACT_BITS: u32 = 8; // Must be less than W_INVERSE_FRACT_BITS and ST_FRACT_BITS

    let approx_reciprocal = ApproxReciprocal::new("approx_reciprocal", W_INVERSE_FRACT_BITS - RESTORED_W_FRACT_BITS - 3, 4, &c);
    sim::generate(approx_reciprocal.m, sim::GenerationOptions::default(), file)
}
