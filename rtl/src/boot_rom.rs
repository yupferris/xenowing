use crate::buster::*;

use kaze::*;

pub struct BootRom<'a> {
    pub m: &'a Module<'a>,
    pub primary_port: PrimaryPort<'a>,
}

impl<'a> BootRom<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> BootRom<'a> {
        let m = p.module(instance_name, "BootRom");

        const CONTENTS_SIZE_BITS: u32 = 12;
        const CONTENTS_SIZE: u32 = 1 << CONTENTS_SIZE_BITS;
        let contents_bytes = {
            let mut ret = include_bytes!("../../sw/boot_rom/boot_rom.bin").iter().cloned().collect::<Vec<u8>>();
            if ret.len() as u32 > CONTENTS_SIZE {
                panic!("Boot ROM cannot be larger than {} bytes", CONTENTS_SIZE);
            }
            // Zero-pad ROM to fill whole size
            while (ret.len() as u32) < CONTENTS_SIZE {
                ret.push(0);
            }
            ret
        };
        let contents = {
            let mut ret = Vec::new();
            for i in 0..CONTENTS_SIZE / 16 {
                let mut value = 0;
                for j in 0..16 {
                    value |= (contents_bytes[(i * 16 + j) as usize] as u128) << (j * 8);
                }
                ret.push(value);
            }
            ret
        };

        let mem = m.mem("mem", CONTENTS_SIZE_BITS - 4, 128);
        mem.initial_contents(&contents);

        let bus_enable = m.input("bus_enable", 1);
        let bus_addr = m.input("bus_addr", 20);
        let bus_write = m.input("bus_write", 1);
        let bus_write_data = m.input("bus_write_data", 128);
        let bus_write_byte_enable = m.input("bus_write_byte_enable", 16);
        let bus_ready = m.output("bus_ready", m.high());
        let read_enable = bus_enable & !bus_write;
        let bus_read_data = m.output("bus_read_data", mem.read_port(bus_addr.bits(CONTENTS_SIZE_BITS - 5, 0), read_enable));
        let bus_read_data_valid = m.output("bus_read_data_valid", read_enable.reg_next_with_default("bus_read_data_valid", false));

        BootRom {
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
        }
    }
}
