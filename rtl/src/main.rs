mod approx_reciprocal;
mod boot_rom;
mod buster;
mod color_thrust;
mod fifo;
mod flow_controlled_pipe;
mod led_interface;
mod marv;
mod marv_interconnect_bridge;
mod mimas_a7;
mod peek_buffer;
mod read_cache;
mod uart;
mod uart_interface;
mod word_mem;
mod xenowing;

use kaze::*;

use std::io::{Result, stdout};

fn main() -> Result<()> {
    let c = Context::new();

    xenowing::generate(&c);
    mimas_a7::test::lfsr::generate(&c);
    mimas_a7::test::uart::generate(&c);

    for m in c.modules().values() {
        verilog::generate(m, stdout())?;
    }

    Ok(())
}
