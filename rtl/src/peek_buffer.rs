use kaze::*;

pub struct PeekBuffer<'a> {
    pub m: &'a Module<'a>,
    pub ingress_data: &'a Input<'a>,
    pub ingress_data_valid: &'a Input<'a>,
    pub ingress_read_enable: &'a Output<'a>,
    pub egress_ready: &'a Output<'a>,
    pub egress_data: &'a Output<'a>,
    pub egress_read_enable: &'a Input<'a>,
}

impl<'a> PeekBuffer<'a> {
    // TODO: Either take in a Fifo to attach to automatically, or add a helper fn to attach to a Fifo
    //  The former is attractive as we can potentially derive data_bit_width from the Fifo parameters (if we store them), which should save boilerplate each time we make one of these!
    //  Maybe it's even more convenient to have a fn on Fifo that creates the correctly parameterized PeekBuffer instance, hooks it up, and returns it?
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, data_bit_width: u32, p: &'a P) -> PeekBuffer<'a> {
        let m = p.module(instance_name, "PeekBuffer");

        let ingress_data = m.input("ingress_data", data_bit_width);
        let ingress_data_valid = m.input("ingress_data_valid", 1);

        let data_buf = m.reg("data_buf", data_bit_width);
        let data_buf_full = m.reg("data_buf_full", 1);
        data_buf_full.default_value(false);

        let egress_ready = m.output("egress_ready", data_buf_full | ingress_data_valid);
        let egress_data = m.output("egress_data", data_buf_full.mux(data_buf, ingress_data));
        let egress_read_enable = m.input("egress_read_enable", 1);

        let ingress_read_enable = m.output("ingress_read_enable", egress_read_enable | (!data_buf_full & !ingress_data_valid));

        // TODO: Can this logic be simplified?
        let next_data_buf = data_buf;
        let next_data_buf_full = data_buf_full;

        let (next_data_buf, next_data_buf_full) = if_(ingress_data_valid & !egress_read_enable, {
            (ingress_data, m.high())
        }).else_({
            (next_data_buf, next_data_buf_full)
        });

        let next_data_buf_full = if_(egress_read_enable, {
            m.low()
        }).else_({
            next_data_buf_full
        });

        data_buf.drive_next(next_data_buf);
        data_buf_full.drive_next(next_data_buf_full);

        PeekBuffer {
            m,
            ingress_data,
            ingress_data_valid,
            ingress_read_enable,
            egress_ready,
            egress_data,
            egress_read_enable,
        }
    }
}
