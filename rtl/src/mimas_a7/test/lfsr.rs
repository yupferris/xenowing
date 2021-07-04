use kaze::*;

pub struct Lfsr<'a> {
    pub m: &'a Module<'a>,
    pub shift_enable: &'a Input<'a>,
    pub value: &'a Output<'a>,
}

impl<'a> Lfsr<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Lfsr<'a> {
        let m = p.module(instance_name, "Lfsr");

        let shift_enable = m.input("shift_enable", 1);

        let state = m.reg("state", 16);
        state.default_value(0xace1u32);
        let value = m.output("value", state.bits(7, 0));

        state.drive_next(if_(shift_enable, {
            let feedback_bit = state.bit(0) ^ state.bit(2) ^ state.bit(3) ^ state.bit(5);
            feedback_bit.concat(state.bits(15, 1))
        }).else_({
            state
        }));

        Lfsr {
            m,
            shift_enable,
            value,
        }
    }
}
