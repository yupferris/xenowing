mod modules {
    include!(concat!(env!("OUT_DIR"), "/modules.rs"));
}

use modules::*;

use kaze::runtime::tracing::*;
use kaze::runtime::tracing::vcd::*;

use rand::{Rng, SeedableRng};
use rand::distributions::{Distribution, Uniform};

use std::env;
use std::fs::File;
use std::io;

fn build_trace(test_name: &'static str) -> io::Result<impl Trace> {
    let mut path = env::temp_dir();
    path.push(format!("{}.vcd", test_name));
    println!("Writing trace to {:?}", path);
    let file = File::create(path)?;
    VcdTrace::new(file, 10, TimeScaleUnit::Ns)
}

fn main() -> io::Result<()> {
    let seed = env::args().skip(1).nth(0).expect("seed not specified").parse().expect("Couldn't parse seed");
    let num_elements = env::args().skip(2).nth(0).expect("num_elements not specified").parse().expect("Couldn't parse num_elements");

    println!("Testing Pipe with seed = {}, num_elements = {}", seed, num_elements);

    let data = (0..num_elements).collect::<Vec<_>>();
    let mut data_write_ptr = 0;
    let mut data_read_ptr = 0;

    let trace = build_trace("Pipe__fuzz")?;

    let mut m = Pipe::new(trace)?;
    let mut time_stamp = 0;

    m.reset();

    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

    loop {
        m.prop();

        // Writes
        if data_write_ptr < data.len() {
            m.in_valid = Uniform::new_inclusive(0.0, 1.0).sample(&mut rng) < 0.75;
            m.in_a = if m.in_valid { data[data_write_ptr] } else { 0xfadebabe };

            if m.in_valid && m.in_ready {
                data_write_ptr += 1;
            }
        } else {
            m.in_valid = false;
            m.in_a = 0xdeadbeef;
        }

        // Aux writes
        m.d = rng.gen();

        // Reads
        m.out_ready = Uniform::new_inclusive(0.0, 1.0).sample(&mut rng) < 0.25;

        m.prop();
        m.update_trace(time_stamp)?;

        if m.out_ready && m.out_valid {
            assert_eq!(m.out_b, data[data_read_ptr]);
            assert_eq!(m.out_c, !data[data_read_ptr]);
            data_read_ptr += 1;
            if data_read_ptr == data.len() {
                break;
            }
        }

        // Aux reads
        assert_eq!(m.e, !m.d);

        m.posedge_clk();
        time_stamp += 1;
    }

    println!("Test successful after {} cycles", time_stamp);

    Ok(())
}
