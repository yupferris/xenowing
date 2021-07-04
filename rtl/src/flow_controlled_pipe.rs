use crate::fifo::*;
use crate::peek_buffer::*;

use kaze::*;

// TODO: UPDATE TO REFLECT ALSO TAKING THE OUTER PIPELINE MODULE AS INPUT
/// Generates a module that wraps an existing inner pipeline module, and adds credit-based flow control logic.
///
/// # Assumptions
///
/// The inner pipeline module (specified by `inner_pipe`) is assumed to be a straightforward pipeline with `inner_pipe_num_stages` stages.
/// Each stage starts by registering its inputs, followed by combinational logic.
///
/// The inner pipeline module can have any nonzero number of inputs and outputs for data, and they need not be symmetrical (eg. a pipe can take 2 input, but produce 3 outputs).
/// In order to facilitate flow control, these inputs and outputs must be forwarded to/from matching signals on the outer pipe module.
/// These signals should be specified with [`input`] and [`output`], respectively.
///
/// In addition to the data inputs/outputs, the pipe is assumed to propagate a `valid` control signal, which communicates whether or not there's valid data at each stage.
/// Like regular inputs/outputs, these must be forwarded to/from matching signals on the outer pipe.
/// Because these signals are known and always required, they must be specified as constructor arguments.
///
/// Optionally, the inner pipeline module may have auxiliary input/output ports that aren't necessarily related to the pipeline data flow.
/// These may be used to interact with other modules for some stages, or some other logic entirely.
/// Like regular inputs/outputs, these must be forwarded to/from matching signals on the outer pipe.
/// These signals should be specified with [`aux_input`] and [`aux_output`], respectively.
/// No additional logic will be generated for these auxiliary signals - they will simply be forwarded from/to matching ports on the generated module.
///
/// # Generated interface
///
/// ## Ports
///
/// Assuming all inputs/outputs have been correctly specified, the generated module will have the same input/output ports (including auxiliary input/output ports) as the inner pipeline.
/// Four additional flow control signals will be added:
///  - `in_ready` - an output signaling that the pipeline can accept data on this cycle
///  - `in_valid` - an input signaling that valid data is presented to the pipeline inputs on this cycle
///  - `out_ready` - an input signaling that valid data presented from the pipeline outputs will be accepted on this cycle
///  - `out_valid` - an output signaling that valid data is presented from the pipeline outputs on this cycle
/// These signals are available as public members of the returned FlowControlledPipe struct.
///
/// ## Handshake
///
/// Data is only accepted into the pipeline inputs for cycles where both `in_ready` and `in_valid` are high.
/// Likewise, data is only accepted from the pipeline outputs for cycles where both `out_ready` and `out_valid` are high.
///
/// ## Latency
///
/// The generated pipeline inserts some (unspecified) number of additional stages to facilitate flow control.
/// Because of this, the generated pipeline's latency will be strictly greater than the inner pipeline's latency.
/// If `out_ready` is always high, the pipeline will behave exactly as the input pipeline with some additional latency.
///
/// ## Throughput
///
/// The generated pipeline will be able to sustain the full throughput of the input pipeline as long as there aren't stalls or bubbles generated in response to the flow control signals.
///
/// [`input`]: Self::input
/// [`output`]: Self::output
/// [`aux_input`]: Self::aux_input
/// [`aux_output`]: Self::aux_output
pub struct FlowControlledPipe<'a> {
    /// The outer pipeline module.
    pub m: &'a Module<'a>,

    /// TODO: Proper doc :)
    pub in_ready: &'a Output<'a>,
    /// TODO: Proper doc :)
    pub in_valid: &'a Input<'a>,
    /// TODO: Proper doc :)
    pub out_ready: &'a Input<'a>,
    /// TODO: Proper doc :)
    // TODO: It would be really great if we could remove this Option!
    pub out_valid: Option<&'a Output<'a>>,

    inner_pipe_out_valid: &'a Output<'a>,
    num_credit_stages: u32,
    num_credits_bit_width: u32,
    num_credits: &'a Register<'a>,
    in_handshake: &'a Signal<'a>,

    has_output: bool,
}

impl<'a> FlowControlledPipe<'a> {
    pub fn new(
        m: &'a Module<'a>,
        inner_pipe_num_stages: u32,
        inner_pipe_in_valid: &'a Input<'a>,
        inner_pipe_out_valid: &'a Output<'a>,
    ) -> FlowControlledPipe<'a> {
        let num_credit_stages = inner_pipe_num_stages; // TODO: Make adjustable?
        let max_num_credits = inner_pipe_num_stages + 3 + num_credit_stages; // TODO: Document derivation

        let num_credits_bit_width = (max_num_credits as f64).log2().ceil() as u32 + 1; // TODO: Use helpers here?
        let num_credits = m.reg("num_credits", num_credits_bit_width);
        num_credits.default_value(max_num_credits);

        // Ingress
        let in_ready = num_credits.ne(m.lit(0u32, num_credits_bit_width));
        let in_valid = m.input("in_valid", 1);

        let in_handshake = in_ready & in_valid;
        inner_pipe_in_valid.drive(in_handshake);

        let out_ready = m.input("out_ready", 1);

        FlowControlledPipe {
            m,

            in_ready: m.output("in_ready", in_ready),
            in_valid,
            out_ready,
            out_valid: None,

            inner_pipe_out_valid,
            num_credit_stages,
            num_credits_bit_width,
            num_credits,
            in_handshake,

            has_output: false,
        }
    }

    /// Specifies an input to the inner pipeline module.
    ///
    /// An input port will be added to the generated module that forwards data directly to the corresponding inner pipeline module's input.
    pub fn input<S: Into<String>>(&mut self, name: S, i: &'a Input<'a>) -> &'a Input<'a> {
        let input = self.m.input(name, i.bit_width());
        i.drive(input);
        input
    }

    /// Specifies an output from the inner pipeline module.
    ///
    /// An output port will be added to the generated module that forwards data directly from the corresponding inner pipeline module's output.
    pub fn output<S: Into<String>>(&mut self, name: S, o: &'a Output<'a>) -> &'a Output<'a> {
        let name = name.into();

        // FIFO
        let fifo_depth_bit_width = self.num_credits_bit_width - 1;
        let fifo = Fifo::new(format!("{}_fifo", name), fifo_depth_bit_width, o.bit_width(), self.m);
        fifo.write_enable.drive(self.inner_pipe_out_valid);
        fifo.write_data.drive(o);

        // Peek buffer
        let peek_buffer = PeekBuffer::new(format!("{}_peek_buffer", name), o.bit_width(), self.m);
        fifo.read_enable.drive(peek_buffer.ingress_read_enable);
        peek_buffer.ingress_data.drive(fifo.read_data);
        peek_buffer.ingress_data_valid.drive(
            (!fifo.empty & peek_buffer.ingress_read_enable)
            .reg_next_with_default(format!("{}_peek_buffer_ingress_data_valid", name), false));

        // Egress
        let output = self.m.output(name, peek_buffer.egress_data);
        let out_valid = peek_buffer.egress_ready;
        peek_buffer.egress_read_enable.drive(self.out_ready);

        if !self.has_output {
            self.out_valid = Some(self.m.output("out_valid", out_valid));

            let out_handshake = self.out_ready & out_valid;

            // Credit return path
            let mut credit_inc = out_handshake;

            for i in 0..self.num_credit_stages {
                credit_inc = credit_inc.reg_next_with_default(format!("stage{}_credit_inc", i), false);
            }

            // Credit adjustment
            let credit_dec = self.in_handshake;
            let credit_adjust_bits = credit_inc.concat(credit_dec);
            self.num_credits.drive_next(if_(credit_adjust_bits.eq(self.m.lit(0b10u32, 2)), {
                self.num_credits + self.m.lit(1u32, self.num_credits_bit_width)
            }).else_if(credit_adjust_bits.eq(self.m.lit(0b01u32, 2)), {
                self.num_credits - self.m.lit(1u32, self.num_credits_bit_width)
            }).else_({
                self.num_credits
            }));

            self.has_output = true;
        }

        output
    }

    /// Specifies an auxiliary input to the inner pipeline module.
    ///
    /// An input port will be added to the generated module that forwards data directly to the corresponding inner pipeline module's input.
    pub fn aux_input<S: Into<String>>(&mut self, name: S, i: &'a Input<'a>) -> &'a Input<'a> {
        let input = self.m.input(name, i.bit_width());
        i.drive(input);
        input
    }

    /// Specifies an auxiliary output from the inner pipeline module.
    ///
    /// An output port will be added to the generated module that forwards data directly from the corresponding inner pipeline module's output.
    pub fn aux_output<S: Into<String>>(&mut self, name: S, o: &'a Output<'a>) -> &'a Output<'a> {
        self.m.output(name, o)
    }
}
