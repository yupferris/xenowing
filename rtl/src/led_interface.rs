use crate::buster::*;

use kaze::*;

pub struct LedInterface<'a> {
    pub m: &'a Module<'a>,
    pub primary_port: PrimaryPort<'a>,
    pub leds: &'a Output<'a>,
}

impl<'a> LedInterface<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> LedInterface<'a> {
        let m = p.module(instance_name, "LedInterface");

        let leds = m.reg("leds", 8);
        leds.default_value(0u32);

        let bus_enable = m.input("bus_enable", 1);
        let bus_addr = m.input("bus_addr", 20);
        let bus_write = m.input("bus_write", 1);
        let bus_write_data = m.input("bus_write_data", 128);
        let bus_write_byte_enable = m.input("bus_write_byte_enable", 16);
        let bus_ready = m.output("bus_ready", m.high());
        let bus_read_data = m.output("bus_read_data", m.lit(0u32, 120).concat(leds));
        let bus_read_data_valid = m.output("bus_read_data_valid", (bus_enable & !bus_write).reg_next_with_default("bus_read_data_valid", false));

        leds.drive_next(if_(bus_enable & bus_write, {
            bus_write_data.bits(7, 0)
        }).else_({
            leds
        }));

        LedInterface {
            m,
            primary_port: PrimaryPort {
                bus_enable,
                bus_addr,
                bus_write,
                bus_write_data,
                bus_write_byte_enable,
                bus_ready,
                bus_read_data,
                bus_read_data_valid,
            },
            leds: m.output("leds", leds),
        }
    }
}
