use crate::buster::*;

use kaze::*;

struct Instruction<'a> {
    pub value: &'a Signal<'a>,
}

impl<'a> Instruction<'a> {
    fn new<S: Into<&'a Signal<'a>>>(value: S) -> Instruction<'a> {
        let value = value.into();

        if value.bit_width() != 32 {
            panic!("value bit width must be 32");
        }

        Instruction {
            value,
        }
    }

    fn opcode(&self) -> &'a Signal<'a> {
        self.value.bits(6, 2) // Bottom two bits are always 0b11 for RV32I, so just ignore them
    }

    fn rs1(&self) -> &'a Signal<'a> {
        self.value.bits(19, 15)
    }

    fn rs2(&self) -> &'a Signal<'a> {
        self.value.bits(24, 20)
    }

    fn rd(&self) -> &'a Signal<'a> {
        self.value.bits(11, 7)
    }

    fn funct3(&self) -> &'a Signal<'a> {
        self.value.bits(14, 12)
    }

    fn load_offset(&self) -> &'a Signal<'a> {
        self.value.bit(31).repeat(21).concat(self.value.bits(30, 20))
    }

    fn store_offset(&self) -> &'a Signal<'a> {
        self.value.bit(31).repeat(21).concat(self.value.bits(30, 25)).concat(self.value.bits(11, 7))
    }

    fn jump_offset(&self, m: &'a Module<'a>) -> &'a Signal<'a> {
        self.value.bit(31).repeat(12).concat(self.value.bits(19, 12)).concat(self.value.bit(20)).concat(self.value.bits(30, 21)).concat(m.low())
    }

    fn branch_offset(&self, m: &'a Module<'a>) -> &'a Signal<'a> {
        self.value.bit(31).repeat(20).concat(self.value.bit(7)).concat(self.value.bits(30, 25)).concat(self.value.bits(11, 8)).concat(m.low())
    }

    fn i_immediate(&self) -> &'a Signal<'a> {
        self.value.bit(31).repeat(21).concat(self.value.bits(30, 20))
    }

    fn u_immediate(&self, m: &'a Module<'a>) -> &'a Signal<'a> {
        self.value.bits(31, 12).concat(m.lit(0u32, 12))
    }

    fn csr(&self) -> &'a Signal<'a> {
        self.value.bits(31, 20)
    }
}

pub struct Marv<'a> {
    pub m: &'a Module<'a>,

    pub replica_port: ReplicaPort<'a>,
}

impl<'a> Marv<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Marv<'a> {
        let m = p.module(instance_name, "Marv");

        let control = Control::new("control", m);

        let register_file = m.mem("register_file", 5, 32);
        register_file.initial_contents(&[0u32; 32]);

        let pc = m.reg("pc", 32);
        pc.default_value(0x00000000u32);

        let cycle_counter = m.reg("cycle_counter", 64);
        cycle_counter.default_value(0u64);
        cycle_counter.drive_next(cycle_counter + m.lit(1u64, 64));

        let instructions_retired_counter = m.reg("instructions_retired_counter", 64);
        instructions_retired_counter.default_value(0u64);

        let bus_ready = m.input("bus_ready", 1);
        let bus_read_data = m.input("bus_read_data", 32);
        let bus_read_data_valid = m.input("bus_read_data_valid", 1);

        let instruction_fetch = InstructionFetch::new("instruction_fetch", m);
        control.instruction_fetch_ready.drive(instruction_fetch.ready);
        instruction_fetch.enable.drive(control.instruction_fetch_enable);
        instruction_fetch.pc.drive(pc.bits(31, 2));
        instruction_fetch.bus_ready.drive(bus_ready);

        let decode = Decode::new("decode", m);
        control.decode_ready.drive(decode.ready);
        decode.bus_read_data.drive(bus_read_data);
        decode.bus_read_data_valid.drive(bus_read_data_valid);

        let decode_instruction = Instruction::new(decode.instruction);

        let instruction = m.reg("instruction", 32);
        instruction.drive_next(control.decode_enable.mux(decode_instruction.value, instruction));
        let instruction = Instruction::new(instruction);

        let alu = Alu::new("alu", m);

        let execute = Execute::new("execute", m);
        execute.pc.drive(pc);
        execute.instruction.drive(instruction.value);
        execute.reg1.drive(register_file.read_port(decode_instruction.rs1(), control.decode_enable));
        execute.reg2.drive(register_file.read_port(decode_instruction.rs2(), control.decode_enable));
        alu.op.drive(execute.alu_op);
        alu.op_mod.drive(execute.alu_op_mod);
        alu.lhs.drive(execute.alu_lhs);
        alu.rhs.drive(execute.alu_rhs);
        execute.alu_res.drive(alu.res);
        execute.cycle_counter_value.drive(cycle_counter);
        execute.instructions_retired_counter_value.drive(instructions_retired_counter);

        let mem = Mem::new("mem", m);
        control.mem_ready.drive(mem.ready);
        mem.enable.drive(control.mem_enable);
        mem.bus_ready_in.drive(bus_ready);
        mem.bus_enable_in.drive(execute.bus_enable);
        mem.bus_addr_in.drive(execute.bus_addr);
        mem.bus_write_data_in.drive(execute.bus_write_data);
        mem.bus_write_byte_enable_in.drive(execute.bus_write_byte_enable);
        mem.bus_write_in.drive(execute.bus_write);
        let bus_write_data = m.output("bus_write_data", mem.bus_write_data_out);

        let writeback = Writeback::new("writeback", m);
        control.writeback_ready.drive(writeback.ready);
        writeback.enable.drive(control.writeback_enable);
        writeback.instruction.drive(instruction.value);
        writeback.bus_addr_low.drive(mem.bus_addr_out.bits(1, 0));
        writeback.next_pc.drive(execute.next_pc);
        writeback.rd_value_write_enable.drive(execute.rd_value_write_enable);
        writeback.rd_value_write_data.drive(execute.rd_value_write_data);
        pc.drive_next(writeback.pc_write_enable.mux(writeback.pc_write_data, pc));
        instructions_retired_counter.drive_next(
            writeback.instructions_retired_counter_increment_enable.mux(
                instructions_retired_counter + m.lit(1u64, 64),
                instructions_retired_counter));
        register_file.write_port(
            writeback.register_file_write_addr,
            writeback.register_file_write_data,
            writeback.register_file_write_enable);
        writeback.bus_read_data.drive(bus_read_data);
        writeback.bus_read_data_valid.drive(bus_read_data_valid);

        let mem_bus_enable = mem.bus_enable_out;
        let bus_enable = m.output("bus_enable", instruction_fetch.bus_enable | mem_bus_enable);
        let bus_addr = m.output("bus_addr", mem_bus_enable.mux(mem.bus_addr_out.bits(31, 2), instruction_fetch.bus_addr));
        let bus_write_byte_enable = m.output("bus_write_byte_enable", mem_bus_enable.mux(mem.bus_write_byte_enable_out, instruction_fetch.bus_write_byte_enable));
        let bus_write = m.output("bus_write", mem_bus_enable & mem.bus_write_out);

        Marv {
            m,

            replica_port: ReplicaPort {
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

pub struct Control<'a> {
    pub m: &'a Module<'a>,

    pub instruction_fetch_ready: &'a Input<'a>,
    pub decode_ready: &'a Input<'a>,
    pub mem_ready: &'a Input<'a>,
    pub writeback_ready: &'a Input<'a>,

    pub instruction_fetch_enable: &'a Output<'a>,
    pub decode_enable: &'a Output<'a>,
    pub mem_enable: &'a Output<'a>,
    pub writeback_enable: &'a Output<'a>,
}

impl<'a> Control<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Control<'a> {
        let m = p.module(instance_name, "Control");

        let instruction_fetch_ready = m.input("instruction_fetch_ready", 1);
        let decode_ready = m.input("decode_ready", 1);
        let mem_ready = m.input("mem_ready", 1);
        let writeback_ready = m.input("writeback_ready", 1);

        // TODO: Figure out how to use/describe enums properly in kaze!
        let state_bit_width = 3;
        let state_instruction_fetch = 0u32;
        let state_decode = 1u32;
        let state_execute = 2u32;
        let state_mem = 3u32;
        let state_writeback = 4u32;
        let state = m.reg("state", state_bit_width);
        state.default_value(state_instruction_fetch);
        // TODO: (Enum) matching sugar
        state.drive_next(if_(state.eq(m.lit(state_instruction_fetch, state_bit_width)) & instruction_fetch_ready, {
            m.lit(state_decode, state_bit_width)
        }).else_if(state.eq(m.lit(state_decode, state_bit_width)) & decode_ready, {
            m.lit(state_execute, state_bit_width)
        }).else_if(state.eq(m.lit(state_execute, state_bit_width)), {
            m.lit(state_mem, state_bit_width)
        }).else_if(state.eq(m.lit(state_mem, state_bit_width)) & mem_ready, {
            m.lit(state_writeback, state_bit_width)
        }).else_if(state.eq(m.lit(state_writeback, state_bit_width)) & writeback_ready, {
            m.lit(state_instruction_fetch, state_bit_width)
        }).else_({
            state
        }));

        let instruction_fetch_enable = m.output("instruction_fetch_enable", state.eq(m.lit(state_instruction_fetch, state_bit_width)));
        let decode_enable = m.output("decode_enable", state.eq(m.lit(state_decode, state_bit_width)));
        let mem_enable = m.output("mem_enable", state.eq(m.lit(state_mem, state_bit_width)));
        let writeback_enable = m.output("writeback_enable", state.eq(m.lit(state_writeback, state_bit_width)));

        Control {
            m,

            instruction_fetch_ready,
            decode_ready,
            mem_ready,
            writeback_ready,

            instruction_fetch_enable,
            decode_enable,
            mem_enable,
            writeback_enable,
        }
    }
}

pub struct InstructionFetch<'a> {
    pub m: &'a Module<'a>,

    pub enable: &'a Input<'a>,
    pub ready: &'a Output<'a>,

    pub pc: &'a Input<'a>,
    pub bus_ready: &'a Input<'a>,
    pub bus_enable: &'a Output<'a>,
    pub bus_addr: &'a Output<'a>,
    pub bus_write_byte_enable: &'a Output<'a>,
}

impl<'a> InstructionFetch<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> InstructionFetch<'a> {
        let m = p.module(instance_name, "InstructionFetch");

        let bus_ready = m.input("bus_ready", 1);
        let ready = m.output("ready", bus_ready);
        let enable = m.input("enable", 1);
        let bus_enable = m.output("bus_enable", enable);
        let pc = m.input("pc", 30);
        let bus_addr = m.output("bus_addr", pc);
        let bus_write_byte_enable = m.output("bus_write_byte_enable", m.high().repeat(4));

        InstructionFetch {
            m,
        
            enable,
            ready,
        
            pc,
            bus_ready,
            bus_enable,
            bus_addr,
            bus_write_byte_enable,
        }
    }
}

pub struct Decode<'a> {
    pub m: &'a Module<'a>,

    pub ready: &'a Output<'a>,

    pub bus_read_data: &'a Input<'a>,
    pub bus_read_data_valid: &'a Input<'a>,
    pub instruction: &'a Output<'a>,
}

impl<'a> Decode<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Decode<'a> {
        let m = p.module(instance_name, "Decode");

        let bus_read_data = m.input("bus_read_data", 32);
        let bus_read_data_valid = m.input("bus_read_data_valid", 1);
        let ready = m.output("ready", bus_read_data_valid);
        let instruction = m.output("instruction", bus_read_data);

        Decode {
            m,

            ready,

            bus_read_data,
            bus_read_data_valid,
            instruction,
        }
    }
}

pub struct Alu<'a> {
    pub m: &'a Module<'a>,

    pub lhs: &'a Input<'a>,
    pub rhs: &'a Input<'a>,
    pub op: &'a Input<'a>,
    pub op_mod: &'a Input<'a>,
    pub res: &'a Output<'a>,
}

impl<'a> Alu<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Alu<'a> {
        let m = p.module(instance_name, "Alu");

        let lhs = m.input("lhs", 32);
        let rhs = m.input("rhs", 32);
        let op = m.input("op", 3);
        let op_mod = m.input("op_mod", 1);

        let shift_amt = rhs.bits(4, 0);

        let res = m.output("res", if_(op.eq(m.lit(0b000u32, 3)), {
            if_(!op_mod, {
                // add
                lhs + rhs
            }).else_({
                // sub
                lhs - rhs
            })
        }).else_if(op.eq(m.lit(0b001u32, 3)), {
            // sll
            lhs << shift_amt
        }).else_if(op.eq(m.lit(0b010u32, 3)), {
            // lt
            m.lit(0u32, 31).concat(lhs.lt_signed(rhs))
        }).else_if(op.eq(m.lit(0b011u32, 3)), {
            // ltu
            m.lit(0u32, 31).concat(lhs.lt(rhs))
        }).else_if(op.eq(m.lit(0b100u32, 3)), {
            // xor
            lhs ^ rhs
        }).else_if(op.eq(m.lit(0b101u32, 3)), {
            if_(!op_mod, {
                // srl
                lhs >> shift_amt
            }).else_({
                // sra
                lhs.shr_arithmetic(shift_amt)
            })
        }).else_if(op.eq(m.lit(0b110u32, 3)), {
            // or
            lhs | rhs
        }).else_({
            // and
            lhs & rhs
        }));

        Alu {
            m,

            lhs,
            rhs,
            op,
            op_mod,
            res,
        }
    }
}

pub struct Execute<'a> {
    pub m: &'a Module<'a>,

    pub pc: &'a Input<'a>,
    pub instruction: &'a Input<'a>,
    pub reg1: &'a Input<'a>,
    pub reg2: &'a Input<'a>,
    pub cycle_counter_value: &'a Input<'a>,
    pub instructions_retired_counter_value: &'a Input<'a>,
    pub next_pc: &'a Output<'a>,
    pub rd_value_write_enable: &'a Output<'a>,
    pub rd_value_write_data: &'a Output<'a>,
    pub bus_enable: &'a Output<'a>,
    pub bus_write: &'a Output<'a>,
    pub bus_addr: &'a Output<'a>,
    pub bus_write_data: &'a Output<'a>,
    pub bus_write_byte_enable: &'a Output<'a>,

    pub alu_lhs: &'a Output<'a>,
    pub alu_rhs: &'a Output<'a>,
    pub alu_op: &'a Output<'a>,
    pub alu_op_mod: &'a Output<'a>,
    pub alu_res: &'a Input<'a>,
}

impl<'a> Execute<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Execute<'a> {
        let m = p.module(instance_name, "Execute");

        let instruction_input = m.input("instruction", 32);
        let instruction = Instruction::new(instruction_input);

        let reg1 = m.input("reg1", 32);
        let reg2 = m.input("reg2", 32);

        let alu_lhs = m.output("alu_lhs", reg1);
        let alu_op = m.output("alu_op", instruction.funct3());

        let alu_op_mod = instruction.value.bit(30);
        let (alu_rhs, alu_op_mod) = if_(instruction.opcode().bit(3), {
            // Register computation
            (reg2, alu_op_mod)
        }).else_({
            // Immediate computation
            //  These use the alu_op_mod bit as part of the immediate operand except for SRAI
            (instruction.i_immediate(), instruction.funct3().eq(m.lit(0b101u32, 3)) & alu_op_mod)
        });

        let alu_rhs = m.output("alu_rhs", alu_rhs);
        let alu_op_mod = m.output("alu_op_mod", alu_op_mod);

        let pc = m.input("pc", 32);
        let link_pc = pc + m.lit(4u32, 32);
        let alu_res = m.input("alu_res", 32);

        let (next_pc, rd_value_write_data) = if_(instruction.opcode().eq(m.lit(0b01101u32, 5)), {
            // lui
            (link_pc, instruction.u_immediate(m))
        }).else_if(instruction.opcode().eq(m.lit(0b00101u32, 5)), {
            // auipc
            (link_pc, instruction.u_immediate(m) + pc)
        }).else_if(instruction.opcode().eq(m.lit(0b11011u32, 5)), {
            // jal
            (pc + instruction.jump_offset(m), link_pc)
        }).else_if(instruction.opcode().eq(m.lit(0b11001u32, 5)), {
            // jalr
            (reg1 + instruction.i_immediate(), link_pc)
        }).else_({
            (link_pc, alu_res)
        });

        // Loads
        let bus_enable = instruction.opcode().eq(m.lit(0b00000u32, 5));

        let bus_addr_offset = instruction.load_offset();

        // Stores
        let (rd_value_write_enable, bus_addr_offset, bus_enable, bus_write) = if_(instruction.opcode().eq(m.lit(0b01000u32, 5)), {
            (m.low(), instruction.store_offset(), m.high(), m.high())
        }).else_({
            (m.high(), bus_addr_offset, bus_enable, m.low())
        });

        let bus_enable = m.output("bus_enable", bus_enable);
        let bus_write = m.output("bus_write", bus_write);

        let bus_addr = reg1 + bus_addr_offset;

        let (bus_write_data, bus_write_byte_enable) = if_(instruction.funct3().bits(1, 0).eq(m.lit(0b00u32, 2)), {
            // sb
            let bus_addr_low = bus_addr.bits(1, 0);
            // TODO: Express with shift?
            if_(bus_addr_low.eq(m.lit(0b00u32, 2)), {
                (reg2.into(), m.lit(0b0001u32, 4))
            }).else_if(bus_addr_low.eq(m.lit(0b01u32, 2)), {
                (m.lit(0u32, 16).concat(reg2.bits(7, 0)).concat(m.lit(0u32, 8)), m.lit(0b0010u32, 4))
            }).else_if(bus_addr_low.eq(m.lit(0b10u32, 2)), {
                (m.lit(0u32, 8).concat(reg2.bits(7, 0)).concat(m.lit(0u32, 16)), m.lit(0b0100u32, 4))
            }).else_({
                (reg2.bits(7, 0).concat(m.lit(0u32, 24)), m.lit(0b1000u32, 4))
            })
        }).else_if(instruction.funct3().bits(1, 0).eq(m.lit(0b01u32, 2)), {
            // sh
            // TODO: Express with shift?
            if_(!bus_addr.bit(1), {
                (reg2, m.lit(0b0011u32, 4))
            }).else_({
                (reg2.bits(15, 0).concat(m.lit(0u32, 16)), m.lit(0b1100u32, 4))
            })
        }).else_({
            // sw
            (reg2, m.lit(0b1111u32, 4))
        });

        let bus_write_data = m.output("bus_write_data", bus_write_data);
        let bus_write_byte_enable = m.output("bus_write_byte_enable", bus_write_byte_enable);

        // Branch instructions
        let funct3_low = instruction.funct3().bits(2, 1);
        // TODO: switch/case construct?
        let branch_taken = if_(funct3_low.eq(m.lit(0b00u32, 2)), {
            reg1.eq(reg2)
        }).else_if(funct3_low.eq(m.lit(0b01u32, 2)), {
            m.low()
        }).else_if(funct3_low.eq(m.lit(0b10u32, 2)), {
            reg1.lt_signed(reg2)
        }).else_({
            reg1.lt(reg2)
        });
        // TODO: Conditional invert construct?
        let branch_taken = instruction.funct3().bit(0).mux(!branch_taken, branch_taken);
        let (rd_value_write_enable, next_pc) = if_(instruction.opcode().eq(m.lit(0b11000u32, 5)), {
            (m.low(), if_(branch_taken, {
                pc + instruction.branch_offset(m)
            }).else_({
                next_pc
            }))
        }).else_({
            (rd_value_write_enable, next_pc)
        });

        let next_pc = m.output("next_pc", next_pc);

        // Fence instructions
        let rd_value_write_enable = if_(instruction.opcode().eq(m.lit(0b00011u32, 5)), {
            // Do nothing (nop)
            m.low()
        }).else_({
            rd_value_write_enable
        });

        let cycle_counter_value = m.input("cycle_counter_value", 64);
        let instructions_retired_counter_value = m.input("instructions_retired_counter_value", 64);

        // System instructions
        let (rd_value_write_enable, rd_value_write_data) = if_(instruction.opcode().eq(m.lit(0b11100u32, 5)), {
            let rd_value_write_enable = if_(instruction.funct3().eq(m.lit(0b000u32, 3)), {
                // ecall/ebreak: do nothing (nop)
                m.low()
            }).else_({
                rd_value_write_enable
            });

            let rd_value_write_data = if_(instruction.funct3().bits(1, 0).ne(m.lit(0b00u32, 2)), {
                // csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci
                let csr_low = instruction.csr().bits(1, 0);
                if_(csr_low.eq(m.lit(0b00u32, 2)) | csr_low.eq(m.lit(0b01u32, 2)), {
                    // cycle, time
                    if_(!instruction.csr().bit(7), {
                        cycle_counter_value.bits(31, 0)
                    }).else_({
                        // cycleh, timeh
                        cycle_counter_value.bits(63, 32)
                    })
                }).else_if(csr_low.eq(m.lit(0b10u32, 2)), {
                    // instret
                    if_(!instruction.csr().bit(7), {
                        instructions_retired_counter_value.bits(31, 0)
                    }).else_({
                        // instreth
                        instructions_retired_counter_value.bits(63, 32)
                    })
                }).else_({
                    rd_value_write_data
                })
            }).else_({
                rd_value_write_data
            });

            (rd_value_write_enable, rd_value_write_data)
        }).else_({
            (rd_value_write_enable, rd_value_write_data)
        });

        let rd_value_write_enable = m.output("rd_value_write_enable", rd_value_write_enable);
        let rd_value_write_data = m.output("rd_value_write_data", rd_value_write_data);

        Execute {
            m,

            pc,
            instruction: instruction_input,
            reg1,
            reg2,
            cycle_counter_value,
            instructions_retired_counter_value,
            next_pc,
            rd_value_write_enable,
            rd_value_write_data,
            bus_enable,
            bus_write,
            bus_addr: m.output("bus_addr", bus_addr),
            bus_write_data,
            bus_write_byte_enable,

            alu_lhs,
            alu_rhs,
            alu_op,
            alu_op_mod,
            alu_res,
        }
    }
}
    

// TODO: I got lazy here; match the pattern used in other stages
struct Mem<'a> {
    pub m: &'a Module<'a>,

    pub enable: &'a Input<'a>,
    pub ready: &'a Output<'a>,

    pub bus_enable_in: &'a Input<'a>,
    pub bus_ready_in: &'a Input<'a>,
    pub bus_addr_in: &'a Input<'a>,
    pub bus_write_data_in: &'a Input<'a>,
    pub bus_write_byte_enable_in: &'a Input<'a>,
    pub bus_write_in: &'a Input<'a>,
    pub bus_enable_out: &'a Output<'a>,
    pub bus_addr_out: &'a Output<'a>,
    pub bus_write_data_out: &'a Output<'a>,
    pub bus_write_byte_enable_out: &'a Output<'a>,
    pub bus_write_out: &'a Output<'a>,
}

impl<'a> Mem<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Mem<'a> {
        let m = p.module(instance_name, "Mem");

        let enable = m.input("enable", 1);

        let bus_enable_in = m.input("bus_enable_in", 1);
        let bus_ready_in = m.input("bus_ready_in", 1);
        let bus_addr_in = m.input("bus_addr_in", 32);
        let bus_write_data_in = m.input("bus_write_data_in", 32);
        let bus_write_byte_enable_in = m.input("bus_write_byte_enable_in", 4);
        let bus_write_in = m.input("bus_write_in", 1);

        let bus_enable = bus_enable_in.reg_next_with_default("bus_enable", false);
        let bus_enable_out = m.output("bus_enable_out", enable & bus_enable);
        let bus_addr_out = m.output("bus_addr_out", bus_addr_in.reg_next("bus_addr"));
        let bus_write_data_out = m.output("bus_write_data_out", bus_write_data_in.reg_next("bus_write_data"));
        let bus_write_byte_enable_out = m.output("bus_write_byte_enable_out", bus_write_byte_enable_in.reg_next("bus_write_byte_enable"));
        let bus_write_out = m.output("bus_write_out", bus_write_in.reg_next("bus_write"));

        let ready = m.output("ready", bus_enable.mux(bus_ready_in, m.high()));

        Mem {
            m,

            enable,
            ready,

            bus_enable_in,
            bus_ready_in,
            bus_addr_in,
            bus_write_data_in,
            bus_write_byte_enable_in,
            bus_write_in,
            bus_enable_out,
            bus_addr_out,
            bus_write_data_out,
            bus_write_byte_enable_out,
            bus_write_out,
        }
    }
}

struct Writeback<'a> {
    pub m: &'a Module<'a>,

    pub enable: &'a Input<'a>,
    pub ready: &'a Output<'a>,

    pub instruction: &'a Input<'a>,
    pub bus_addr_low: &'a Input<'a>,
    pub bus_read_data: &'a Input<'a>,
    pub bus_read_data_valid: &'a Input<'a>,
    pub rd_value_write_data: &'a Input<'a>,
    pub rd_value_write_enable: &'a Input<'a>,
    pub next_pc: &'a Input<'a>,
    pub pc_write_data: &'a Output<'a>,
    pub pc_write_enable: &'a Output<'a>,
    pub instructions_retired_counter_increment_enable: &'a Output<'a>,
    pub register_file_write_addr: &'a Output<'a>,
    pub register_file_write_data: &'a Output<'a>,
    pub register_file_write_enable: &'a Output<'a>,
}

impl<'a> Writeback<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> Writeback<'a> {
        let m = p.module(instance_name, "Writeback");

        let enable = m.input("enable", 1);

        let instruction_input = m.input("instruction", 32);
        let instruction = Instruction::new(instruction_input);
        let bus_addr_low = m.input("bus_addr_low", 2);
        let bus_read_data = m.input("bus_read_data", 32);
        let bus_read_data_valid = m.input("bus_read_data_valid", 1);
        let rd_value_write_data = m.input("rd_value_write_data", 32);
        let rd_value_write_enable = m.input("rd_value_write_enable", 1);

        let (ready, register_file_write_data) = if_(instruction.opcode().eq(m.lit(0b00000u32, 5)), {
            // Loads
            let register_file_write_data = if_(instruction.funct3().bits(1, 0).eq(m.lit(0b00u32, 2)), {
                // lb/lbu
                let register_file_write_data = if_(bus_addr_low.eq(m.lit(0b00u32, 2)), {
                    bus_read_data.bit(7).repeat(24).concat(bus_read_data.bits(7, 0))
                }).else_if(bus_addr_low.eq(m.lit(0b01u32, 2)), {
                    bus_read_data.bit(15).repeat(24).concat(bus_read_data.bits(15, 8))
                }).else_if(bus_addr_low.eq(m.lit(0b10u32, 2)), {
                    bus_read_data.bit(23).repeat(24).concat(bus_read_data.bits(23, 16))
                }).else_({
                    bus_read_data.bit(31).repeat(24).concat(bus_read_data.bits(31, 24))
                });

                if_(instruction.funct3().bit(2), {
                    m.lit(0u32, 24).concat(register_file_write_data.bits(7, 0))
                }).else_({
                    register_file_write_data
                })
            }).else_if(instruction.funct3().bits(1, 0).eq(m.lit(0b01u32, 2)), {
                // lh/lhu
                let register_file_write_data = if_(!bus_addr_low.bit(1), {
                    bus_read_data.bit(15).repeat(16).concat(bus_read_data.bits(15, 0))
                }).else_({
                    bus_read_data.bit(31).repeat(16).concat(bus_read_data.bits(31, 16))
                });

                if_(instruction.funct3().bit(2), {
                    m.lit(0u32, 16).concat(register_file_write_data.bits(15, 0))
                }).else_({
                    register_file_write_data
                })
            }).else_({
                // lw
                bus_read_data
            });

            (bus_read_data_valid, register_file_write_data)
        }).else_({
            (m.high(), rd_value_write_data)
        });

        let next_pc = m.input("next_pc", 32);
        let pc_write_data = m.output("pc_write_data", next_pc);
        let pc_write_enable = m.output("pc_write_enable", enable & ready);

        let instructions_retired_counter_increment_enable = m.output("instructions_retired_counter_increment_enable", enable & ready);

        let register_file_write_addr = m.output("register_file_write_addr", instruction.rd());
        let register_file_write_data = m.output("register_file_write_data", register_file_write_data);
        let register_file_write_enable = m.output("register_file_write_enable", enable & ready & rd_value_write_enable & instruction.rd().ne(m.lit(0u32, 5)));

        Writeback {
            m,

            enable,
            ready: m.output("ready", ready),

            instruction: instruction_input,
            bus_addr_low,
            bus_read_data,
            bus_read_data_valid,
            rd_value_write_data,
            rd_value_write_enable,
            next_pc,
            pc_write_data,
            pc_write_enable,
            instructions_retired_counter_increment_enable,
            register_file_write_addr,
            register_file_write_data,
            register_file_write_enable,
        }
    }
}
