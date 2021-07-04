use kaze::*;

pub struct Fifo<'a> {
    pub m: &'a Module<'a>,

    // Writes
    pub full: &'a Output<'a>,
    pub write_enable: &'a Input<'a>,
    pub write_data: &'a Input<'a>,

    // Reads
    pub empty: &'a Output<'a>,
    pub read_enable: &'a Input<'a>,
    pub read_data: &'a Output<'a>,
}

impl<'a> Fifo<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, depth_bit_width: u32, element_bit_width: u32, p: &'a P) -> Fifo<'a> {
        let m = p.module(instance_name, "Fifo");

        let mem = m.mem("mem", depth_bit_width, element_bit_width);

        let count_bits = depth_bit_width + 1;
        let count = m.reg("count", count_bits);
        count.default_value(0u32);

        let next_count = count;

        // Writes
        let full = count.bit(count_bits - 1);

        let write_enable = m.input("write_enable", 1);
        let write_accept = write_enable & !full;

        let mem_write_addr = m.reg("mem_write_addr", depth_bit_width);
        mem_write_addr.default_value(0u32);

        let write_data = m.input("write_data", element_bit_width);
        mem.write_port(mem_write_addr, write_data, write_accept);

        let (next_count, next_mem_write_addr) = if_(write_accept, {
            (next_count + m.lit(1u32, count_bits), mem_write_addr + m.lit(1u32, depth_bit_width))
        }).else_({
            (next_count, mem_write_addr)
        });

        mem_write_addr.drive_next(next_mem_write_addr);

        // Reads
        let empty = count.eq(m.lit(0u32, count_bits));

        let read_enable = m.input("read_enable", 1);
        let read_accept = read_enable & !empty;

        let mem_read_addr = m.reg("mem_read_addr", depth_bit_width);
        mem_read_addr.default_value(0u32);

        let read_data = m.output("read_data", mem.read_port(mem_read_addr, read_accept));

        let (next_count, next_mem_read_addr) = if_(read_accept, {
            (next_count - m.lit(1u32, count_bits), mem_read_addr + m.lit(1u32, depth_bit_width))
        }).else_({
            (next_count, mem_read_addr)
        });

        mem_read_addr.drive_next(next_mem_read_addr);

        count.drive_next(next_count);

        Fifo {
            m,

            // Writes
            full: m.output("full", full),
            write_enable,
            write_data,

            // Reads
            empty: m.output("empty", empty),
            read_enable,
            read_data,
        }
    }
}
