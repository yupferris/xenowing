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

    let inner_pipe_num_stages = 4;
    let data_bit_width = 32;

    // Outer pipe module
    let pipe = c.module("pipe", "Pipe");

    // Inner pipe
    let inner_pipe = InnerPipe::new("inner_pipe", inner_pipe_num_stages, data_bit_width, pipe);

    // Outer pipe
    let mut pipe = flow_controlled_pipe::FlowControlledPipe::new(pipe, inner_pipe_num_stages, inner_pipe.in_valid, inner_pipe.out_valid);
    pipe.input("in_a", inner_pipe.in_a);
    pipe.output("out_b", inner_pipe.out_b);
    pipe.output("out_c", inner_pipe.out_c);
    pipe.aux_input("d", inner_pipe.d);
    pipe.aux_output("e", inner_pipe.e);

    sim::generate(pipe.m, sim::GenerationOptions {
        tracing: true,
        ..Default::default()
    }, file)
}

struct InnerPipe<'a> {
    pub m: &'a Module<'a>,
    pub in_a: &'a Input<'a>,
    pub in_valid: &'a Input<'a>,
    pub out_b: &'a Output<'a>,
    pub out_c: &'a Output<'a>,
    pub out_valid: &'a Output<'a>,
    pub d: &'a Input<'a>,
    pub e: &'a Output<'a>,
}

impl<'a> InnerPipe<'a> {
    fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, num_pipe_stages: u32, data_bit_width: u32, p: &'a P) -> InnerPipe<'a> {
        let m = p.module(instance_name, "InnerPipe");

        // Pipeline
        let in_a = m.input("in_a", data_bit_width);
        let in_valid = m.input("in_valid", 1);
        let mut a: &'a Signal<'a> = in_a.into();
        let mut valid: &'a Signal<'a> = in_valid.into();

        for i in 0..num_pipe_stages {
            a = a.reg_next(format!("stage{}_a", i));
            valid = valid.reg_next_with_default(format!("stage{}_valid", i), false);
        }

        let out_b = m.output("out_b", a);
        let out_c = m.output("out_c", !a);
        let out_valid = m.output("out_valid", valid);

        // Aux
        let d = m.input("d", 1);
        let e = m.output("e", !d);

        InnerPipe {
            m,
            in_a,
            in_valid,
            out_b,
            out_c,
            out_valid,
            d,
            e,
        }
    }
}
