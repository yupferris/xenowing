use crate::buster::*;

use kaze::*;

pub struct MarvInterconnectBridge<'a> {
    pub m: &'a Module<'a>,
    pub primary_port: PrimaryPort<'a>,
    pub replica_port: ReplicaPort<'a>,
}

impl<'a> MarvInterconnectBridge<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> MarvInterconnectBridge<'a> {
        let m = p.module(instance_name, "MarvInterconnectBridge");

        let marv_data_bit_width = 32;
        let marv_addr_bit_width = 30;

        let primary_bus_enable = m.input("primary_bus_enable", 1);
        let primary_bus_addr = m.input("primary_bus_addr", marv_addr_bit_width);
        let primary_bus_write = m.input("primary_bus_write", 1);
        let primary_bus_write_data = m.input("primary_bus_write_data", marv_data_bit_width);
        let primary_bus_write_byte_enable = m.input("primary_bus_write_byte_enable", marv_data_bit_width / 8);

        let replica_bus_enable = m.output("replica_bus_enable", primary_bus_enable);
        let replica_bus_addr = m.output("replica_bus_addr", primary_bus_addr.bits(marv_addr_bit_width - 1, 2));
        let replica_bus_write = m.output("replica_bus_write", primary_bus_write);

        let marv_issue_word_select = primary_bus_addr.bits(1, 0);
        let (replica_bus_write_data, replica_bus_write_byte_enable) = if_(marv_issue_word_select.eq(m.lit(0b00u32, 2)), {
            (m.lit(0u32, 96).concat(primary_bus_write_data), m.lit(0u32, 12).concat(primary_bus_write_byte_enable))
        }).else_if(marv_issue_word_select.eq(m.lit(0b01u32, 2)), {
            (m.lit(0u32, 64).concat(primary_bus_write_data).concat(m.lit(0u32, 32)), m.lit(0u32, 8).concat(primary_bus_write_byte_enable).concat(m.lit(0u32, 4)))
        }).else_if(marv_issue_word_select.eq(m.lit(0b10u32, 2)), {
            (m.lit(0u32, 32).concat(primary_bus_write_data).concat(m.lit(0u32, 64)), m.lit(0u32, 4).concat(primary_bus_write_byte_enable).concat(m.lit(0u32, 8)))
        }).else_({
            (primary_bus_write_data.concat(m.lit(0u32, 96)), primary_bus_write_byte_enable.concat(m.lit(0u32, 12)))
        });

        let replica_bus_read_data = m.input("replica_bus_read_data", 128);

        let read_data_word_select = m.reg("read_data_word_select", 2);
        read_data_word_select.drive_next(if_(primary_bus_enable & !primary_bus_write, {
            primary_bus_addr.bits(1, 0)
        }).else_({
            read_data_word_select
        }));

        let replica_bus_ready = m.input("replica_bus_ready", 1);
        let primary_bus_ready = m.output("primary_bus_ready", replica_bus_ready);
        let primary_bus_read_data = m.output("primary_bus_read_data", if_(read_data_word_select.eq(m.lit(0b00u32, 2)), {
            replica_bus_read_data.bits(31, 0)
        }).else_if(read_data_word_select.eq(m.lit(0b01u32, 2)), {
            replica_bus_read_data.bits(63, 32)
        }).else_if(read_data_word_select.eq(m.lit(0b10u32, 2)), {
            replica_bus_read_data.bits(95, 64)
        }).else_({
            replica_bus_read_data.bits(127, 96)
        }));
        let replica_bus_read_data_valid = m.input("replica_bus_read_data_valid", 1);
        let primary_bus_read_data_valid = m.output("primary_bus_read_data_valid", replica_bus_read_data_valid);

        MarvInterconnectBridge {
            m,
            primary_port: PrimaryPort {
                bus_enable: primary_bus_enable,
                bus_addr: primary_bus_addr,
                bus_write: primary_bus_write,
                bus_write_data: primary_bus_write_data,
                bus_write_byte_enable: primary_bus_write_byte_enable,
                bus_ready: primary_bus_ready,
                bus_read_data: primary_bus_read_data,
                bus_read_data_valid: primary_bus_read_data_valid,
            },
            replica_port: ReplicaPort {
                bus_enable: replica_bus_enable,
                bus_addr: replica_bus_addr,
                bus_write: replica_bus_write,
                bus_write_data: m.output("replica_bus_write_data", replica_bus_write_data),
                bus_write_byte_enable: m.output("replica_bus_write_byte_enable", replica_bus_write_byte_enable),
                bus_ready: replica_bus_ready,
                bus_read_data: replica_bus_read_data,
                bus_read_data_valid: replica_bus_read_data_valid,
            },
        }
    }
}
