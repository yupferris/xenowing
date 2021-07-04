use kaze::*;

pub struct UartRx<'a> {
    pub m: &'a Module<'a>,
    pub rx: &'a Input<'a>,
    pub data: &'a Output<'a>,
    pub data_valid: &'a Output<'a>,
}

impl<'a> UartRx<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, clock_freq: u32, baud_rate: u32, p: &'a P) -> UartRx<'a> {
        let m = p.module(instance_name, "UartRx");

        // Requires external sync FF's
        let rx = m.input("rx", 1);

        // Sample at 4x baud_rate
        //  We should technically only need 2x due to nyquist/shannon, but due to slight rate differences we want some headroom, so we go for 4 instead
        let tick_rate = baud_rate * 4;
        let clocks_per_tick = clock_freq / tick_rate;
        let tick_counter = m.reg("tick_counter", 32);
        tick_counter.default_value(0u32);
        let tick = tick_counter.eq(m.lit(clocks_per_tick - 1, 32));
        tick_counter.drive_next(tick.mux(m.lit(0u32, 32), tick_counter + m.lit(1u32, 32)));

        let wait_counter_bit_width = 2;
        let wait_counter = m.reg("wait_counter", wait_counter_bit_width);
        let next_wait_counter = wait_counter;

        let data = m.reg("data", 8);
        let next_data = data.into();
        let data_valid = m.reg("data_valid", 1);
        data_valid.default_value(false);
        let next_data_valid = m.low();

        let bit_counter = m.reg("bit_counter", 3);
        bit_counter.default_value(0u32);
        let next_bit_counter = bit_counter.into();

        let state_bit_width = 2;
        let state_idle = 0u32;
        let state_start_bit_wait = 1u32;
        let state_input_bit = 2u32;
        let state_stop_bit_wait = 3u32;
        let state = m.reg("state", state_bit_width);
        state.default_value(state_idle);
        let next_state = state;
        let (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state) = if_(tick, {
            let next_wait_counter = wait_counter + m.lit(1u32, wait_counter_bit_width);

            let (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state) = if_(state.eq(m.lit(state_idle, state_bit_width)), {
                if_(!rx, {
                    let next_wait_counter = m.lit(0u32, wait_counter_bit_width);
                    let next_state = m.lit(state_start_bit_wait, state_bit_width);

                    (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
                }).else_({
                    (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
                })
            }).else_if(state.eq(m.lit(state_start_bit_wait, state_bit_width)), {
                let (next_wait_counter, next_state) = if_(wait_counter.eq(m.lit(1u32, wait_counter_bit_width)), {
                    let next_wait_counter = m.lit(0u32, wait_counter_bit_width);
                    let next_state = m.lit(state_input_bit, state_bit_width);

                    (next_wait_counter, next_state)
                }).else_({
                    (next_wait_counter, next_state)
                });

                let next_state = if_(rx, {
                    // Input is probably spurious; go back to idle state
                    m.lit(state_idle, state_bit_width)
                }).else_({
                    next_state
                });

                (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
            }).else_if(state.eq(m.lit(state_input_bit, state_bit_width)), {
                let (next_data, next_data_valid, next_bit_counter, next_state) = if_(wait_counter.eq(m.lit(3u32, wait_counter_bit_width)), {
                    let next_data = rx.concat(data.bits(7, 1));
                    let next_bit_counter = bit_counter + m.lit(1u32, 3);

                    if_(bit_counter.eq(m.lit(7u32, 3)), {
                        let next_data_valid = m.high();
                        let next_state = m.lit(state_stop_bit_wait, state_bit_width);

                        (next_data, next_data_valid, next_bit_counter, next_state)
                    }).else_({
                        (next_data, next_data_valid, next_bit_counter, next_state)
                    })
                }).else_({
                    (next_data, next_data_valid, next_bit_counter, next_state)
                });

                (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
            }).else_({
                let next_state = if_(wait_counter.eq(m.lit(3u32, wait_counter_bit_width)), {
                    m.lit(state_idle, state_bit_width)
                }).else_({
                    next_state
                });

                (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
            });

            (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
        }).else_({
            (next_wait_counter, next_data, next_data_valid, next_bit_counter, next_state)
        });

        wait_counter.drive_next(next_wait_counter);

        data.drive_next(next_data);
        data_valid.drive_next(next_data_valid);

        bit_counter.drive_next(next_bit_counter);

        state.drive_next(next_state);

        UartRx {
            m,
            rx,
            data: m.output("data", data),
            data_valid: m.output("data_valid", data_valid),
        }
    }
}


pub struct UartTx<'a> {
    pub m: &'a Module<'a>,
    pub ready: &'a Output<'a>,
    pub tx: &'a Output<'a>,
    pub enable: &'a Input<'a>,
    pub data: &'a Input<'a>,
}

impl<'a> UartTx<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, clock_freq: u32, baud_rate: u32, p: &'a P) -> UartTx<'a> {
        let m = p.module(instance_name, "UartTx");

        let state_bit_width = 2;
        let state_idle = 0u32;
        let state_start_bit = 1u32;
        let state_bit = 2u32;
        let state_stop_bit = 3u32;
        let state = m.reg("state", state_bit_width);
        state.default_value(state_idle);
        let next_state = state;

        let ready = m.output("ready", state.eq(m.lit(state_idle, state_bit_width)));

        let tx = m.reg("tx", 1);
        tx.default_value(true);
        let next_tx = tx;

        let enable = m.input("enable", 1);
        let data = m.input("data", 8);

        let data_latch = m.reg("data_latch", 8);
        let next_data_latch = data_latch;

        let tick_rate = baud_rate;
        let clocks_per_tick = clock_freq / tick_rate;
        let tick_counter = m.reg("tick_counter", 32);
        tick_counter.default_value(0u32);
        let tick = tick_counter.eq(m.lit(clocks_per_tick - 1, 32));
        // TODO: Reset this counter if we're in the idle state and we accept a new write
        let next_tick_counter = tick.mux(m.lit(0u32, 32), tick_counter + m.lit(1u32, 32));

        let bit_counter = m.reg("bit_counter", 3);
        bit_counter.default_value(0u32);
        let next_bit_counter = bit_counter.into();

        let (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter) = if_(state.eq(m.lit(state_idle, state_bit_width)), {
            if_(enable, {
                let next_state = m.lit(state_start_bit, state_bit_width);
                let next_tx = m.low();
                let next_data_latch = data;
                let next_tick_counter = m.lit(0u32, 32);

                (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
            }).else_({
                (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
            })
        }).else_({
            if_(tick, {
                if_(state.eq(m.lit(state_start_bit, state_bit_width)), {
                    let next_state = m.lit(state_bit, state_bit_width);
                    let next_tx = data_latch.bit(0);
                    let next_data_latch = m.low().concat(data_latch.bits(7, 1));

                    (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
                }).else_if(state.eq(m.lit(state_bit, state_bit_width)), {
                    let next_bit_counter = bit_counter + m.lit(1u32, 3);

                    if_(bit_counter.eq(m.lit(7u32, 3)), {
                        let next_state = m.lit(state_stop_bit, state_bit_width);
                        let next_tx = m.high();

                        (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
                    }).else_({
                        let next_tx = data_latch.bit(0);
                        let next_data_latch = m.low().concat(data_latch.bits(7, 1));

                        (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
                    })
                }).else_({
                    let next_state = m.lit(state_idle, state_bit_width);

                    (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
                })
            }).else_({
                (next_state, next_tx, next_data_latch, next_tick_counter, next_bit_counter)
            })
        });

        state.drive_next(next_state);

        tx.drive_next(next_tx);

        data_latch.drive_next(next_data_latch);

        tick_counter.drive_next(next_tick_counter);

        bit_counter.drive_next(next_bit_counter);

        UartTx {
            m,
            ready,
            tx: m.output("tx", tx),
            enable,
            data,
        }
    }
}
