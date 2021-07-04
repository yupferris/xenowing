use crate::buster::*;
use crate::fifo::*;

use kaze::*;

pub struct UartInterface<'a> {
    pub m: &'a Module<'a>,
    pub primary_port: PrimaryPort<'a>,
    pub rx_data: &'a Input<'a>,
    pub rx_data_valid: &'a Input<'a>,
    pub tx_ready: &'a Input<'a>,
    pub tx_data: &'a Output<'a>,
    pub tx_enable: &'a Output<'a>,
}

impl<'a> UartInterface<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> UartInterface<'a> {
        let m = p.module(instance_name, "UartInterface");

        let rx_data = m.input("rx_data", 8);
        let rx_data_valid = m.input("rx_data_valid", 1);

        let rx_fifo = Fifo::new("rx_fifo", 8, 8, m);
        rx_fifo.write_enable.drive(rx_data_valid);
        rx_fifo.write_data.drive(rx_data);

        let tx_ready = m.input("tx_ready", 1);

        let bus_enable = m.input("bus_enable", 1);
        let bus_addr = m.input("bus_addr", 20);
        let bus_write = m.input("bus_write", 1);
        let bus_write_data = m.input("bus_write_data", 128);
        let bus_write_byte_enable = m.input("bus_write_byte_enable", 16);
        let bus_ready = m.output("bus_ready", m.high());

        rx_fifo.read_enable.drive(bus_enable & !bus_write & bus_addr.bits(1, 0).eq(m.lit(3u32, 2)));

        let bus_read_return_addr = bus_addr.reg_next("bus_read_return_addr");
        let bus_read_data = m.output("bus_read_data", if_(bus_read_return_addr.bits(1, 0).eq(m.lit(0u32, 2)), {
            m.lit(0u32, 127).concat(tx_ready)
        }).else_if(bus_read_return_addr.bits(1, 0).eq(m.lit(1u32, 2)), {
            m.lit(0u32, 128)
        }).else_if(bus_read_return_addr.bits(1, 0).eq(m.lit(2u32, 2)), {
            m.lit(0u32, 127).concat(!rx_fifo.empty)
        }).else_({
            m.lit(0u32, 120).concat(rx_fifo.read_data)
        }));
        let bus_read_data_valid = m.output("bus_read_data_valid", (bus_enable & !bus_write).reg_next_with_default("bus_read_data_valid", false));

        let tx_data = m.output("tx_data", bus_write_data.bits(7, 0));
        let tx_enable = m.output("tx_enable", bus_enable & bus_write);

        UartInterface {
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
            rx_data,
            rx_data_valid,
            tx_ready,
            tx_data,
            tx_enable,
        }
    }
}
