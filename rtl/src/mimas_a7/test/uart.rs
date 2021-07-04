use crate::uart::*;
use super::lfsr::*;

use kaze::*;

pub struct Uart<'a> {
    pub m: &'a Module<'a>,
    pub rx: &'a Input<'a>,
    pub tx: &'a Output<'a>,
    pub has_errored: &'a Output<'a>,
}

impl<'a> Uart<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Uart<'a> {
        let m = p.module(instance_name, "Uart");

        let has_errored = m.reg("has_errored", 1);
        has_errored.default_value(false);

        let write_enable = !has_errored;

        let uart_tx = UartTx::new("uart_tx", 100000000, 460800, p);
        uart_tx.enable.drive(write_enable);
        let tx = m.output("tx", uart_tx.tx);

        let tx_lfsr = Lfsr::new("tx_lfsr", p);
        tx_lfsr.shift_enable.drive(write_enable & uart_tx.ready);
        uart_tx.data.drive(tx_lfsr.value);

        let rx = m.input("rx", 1);
        let uart_rx = UartRx::new("uart_rx", 100000000, 460800, p);
        uart_rx.rx.drive(rx);
        let read_data = uart_rx.data;
        let read_data_valid = uart_rx.data_valid;

        let rx_lfsr = Lfsr::new("rx_lfsr", p);
        rx_lfsr.shift_enable.drive(read_data_valid);

        has_errored.drive_next(if_(read_data_valid, {
            read_data.ne(rx_lfsr.value)
        }).else_({
            has_errored
        }));

        Uart {
            m,
            rx,
            tx,
            has_errored: m.output("has_errored", has_errored),
        }
    }
}
