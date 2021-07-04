use crate::boot_rom::*;
use crate::buster::*;
use crate::color_thrust::*;
use crate::led_interface::*;
use crate::marv::*;
use crate::marv_interconnect_bridge::*;
use crate::uart::*;
use crate::uart_interface::*;
use crate::word_mem::*;

use kaze::*;

pub struct Xenowing<'a> {
    pub m: &'a Module<'a>,
    pub leds: &'a Output<'a>,
    pub tx: &'a Output<'a>,
    pub rx: &'a Input<'a>,
}

impl<'a> Xenowing<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Xenowing<'a> {
        let m = c.module("Xenowing");

        let marv = Marv::new("marv", m);
        let marv_interconnect_bridge = MarvInterconnectBridge::new("marv_interconnect_bridge", m);
        marv_interconnect_bridge.primary_port.connect(&marv.replica_port);

        let boot_rom = BootRom::new("boot_rom", m);

        // TODO: Break out program RAM into its own module; expose buster port
        let program_ram_addr_bit_width = 13;
        let program_ram_bus_enable = interconnect.output("program_ram_bus_enable");
        let program_ram_bus_write = interconnect.output("program_ram_bus_write");
        let program_ram_bus_addr = interconnect.output("program_ram_bus_addr").bits(program_ram_addr_bit_width - 1, 0);
        let program_ram_bus_write_data = interconnect.output("program_ram_bus_write_data");
        let program_ram_bus_write_byte_enable = interconnect.output("program_ram_bus_write_byte_enable");
        interconnect.drive_input("program_ram_bus_ready", m.high());
        let program_ram_mem = WordMem::new(m, "program_ram_mem", program_ram_addr_bit_width, 8, 16);
        program_ram_mem.write_port(program_ram_bus_addr, program_ram_bus_write_data, program_ram_bus_enable & program_ram_bus_write, program_ram_bus_write_byte_enable);
        interconnect.drive_input("program_ram_bus_read_data", program_ram_mem.read_port(program_ram_bus_addr, program_ram_bus_enable & !program_ram_bus_write));
        interconnect.drive_input("program_ram_bus_read_data_valid", (program_ram_bus_enable & !program_ram_bus_write).reg_next_with_default("program_ram_bus_read_data_valid", false));

        let led_interface = LedInterface::new("led_interface", m);
        let leds = m.output("leds", led_interface.leds);

        let uart_tx = UartTx::new("uart_tx", 100000000, 460800, m);
        let tx = m.output("tx", uart_tx.tx);

        let uart_rx = UartRx::new("uart_rx", 100000000, 460800, m);
        let rx = m.input("rx", 1);
        uart_rx.rx.drive(rx);

        let uart_interface = UartInterface::new("uart_interface", m);
        uart_tx.data.drive(uart_interface.tx_data);
        uart_tx.enable.drive(uart_interface.tx_enable);
        uart_interface.tx_ready.drive(uart_tx.ready);
        uart_interface.rx_data.drive(uart_rx.data);
        uart_interface.rx_data_valid.drive(uart_rx.data_valid);

        let color_thrust = ColorThrust::new("color_thrust", m);

        // TODO: Break out DDR3 interface into its own module; expose buster port
        let ddr3_interface_addr_bit_width = 13;
        let ddr3_interface_bus_enable = interconnect.output("ddr3_interface_bus_enable");
        let ddr3_interface_bus_write = interconnect.output("ddr3_interface_bus_write");
        let ddr3_interface_bus_addr = interconnect.output("ddr3_interface_bus_addr").bits(ddr3_interface_addr_bit_width - 1, 0);
        let ddr3_interface_bus_write_data = interconnect.output("ddr3_interface_bus_write_data");
        let ddr3_interface_bus_write_byte_enable = interconnect.output("ddr3_interface_bus_write_byte_enable");
        interconnect.drive_input("ddr3_interface_bus_ready", m.high());
        let ddr3_mem = WordMem::new(m, "ddr3_mem", ddr3_interface_addr_bit_width, 8, 16);
        ddr3_mem.write_port(ddr3_interface_bus_addr, ddr3_interface_bus_write_data, ddr3_interface_bus_enable & ddr3_interface_bus_write, ddr3_interface_bus_write_byte_enable);
        interconnect.drive_input("ddr3_interface_bus_read_data", ddr3_mem.read_port(ddr3_interface_bus_addr, ddr3_interface_bus_enable & !ddr3_interface_bus_write));
        interconnect.drive_input("ddr3_interface_bus_read_data_valid", (ddr3_interface_bus_enable & !ddr3_interface_bus_write).reg_next_with_default("ddr3_interface_bus_read_data_valid", false));

        // Interconnect
        let cpu_crossbar = Crossbar::new("cpu_crossbar", 1, 2, 28, 4, 128, 5, m);

        // I'm not sure I'm sold on the names primary/replica port anymore!
        // And I'm definitely not sold on the fact that primary ports are the ones we call "connect" on now, seeing this..
        cpu_crossbar.primary_ports[0].connect(&marv_interconnect_bridge.replica_port);

        // TODO: Better name?
        let mem_crossbar = Crossbar::new("mem_crossbar", 2, 1, 13, 0, 128, 5, m);
        mem_crossbar.primary_ports[0].connect(&cpu_crossbar.replica_ports[1]);
        mem_crossbar.primary_ports[1].connect(&color_thrust.replica_port);
        ddr3_interface.primary_port.connect(&mem_crossbar.replica_ports[0]);

        let sys_crossbar = Crossbar::new("buster_crossbar", 1, 7, 24, 4, 128, 5, m);
        sys_crossbar.primary_ports[0].connect(&cpu_crossbar.replica_ports[0]);
        boot_rom.primary_port.connect(&sys_crossbar.replica_ports[0]);
        program_ram.primary_port.connect(&sys_crossbar.replica_ports[1]);
        led_interface.primary_port.connect(&sys_crossbar.replica_ports[2]);
        uart_interface.primary_port.connect(&sys_crossbar.replica_ports[3]);
        color_thrust.reg_primary_port.connect(&sys_crossbar.replica_ports[4]);
        color_thrust.color_buffer_primary_port.connect(&sys_crossbar.replica_ports[5]);
        color_thrust.depth_buffer_primary_port.connect(&sys_crossbar.replica_ports[6]);

        Xenowing {
            m,
            leds,
            tx,
            rx,
        }
    }
}
