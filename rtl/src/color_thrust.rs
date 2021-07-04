mod tex_cache;

use crate::approx_reciprocal::*;
use crate::buster::*;
use crate::flow_controlled_pipe::*;
use crate::word_mem::*;

use kaze::*;

pub const TILE_DIM_BITS: u32 = 4;
pub const TILE_DIM: u32 = 1 << TILE_DIM_BITS;
pub const TILE_PIXELS_BITS: u32 = TILE_DIM_BITS * 2;
pub const TILE_PIXELS: u32 = 1 << TILE_PIXELS_BITS;
pub const TILE_PIXELS_WORDS_BITS: u32 = TILE_PIXELS_BITS - 2;

pub const TEX_PIXEL_ADDR_BITS: u32 = 17 - 2;
pub const TEX_WORD_ADDR_BITS: u32 = TEX_PIXEL_ADDR_BITS - 2;

pub const EDGE_FRACT_BITS: u32 = 8;
pub const COLOR_WHOLE_BITS: u32 = 9;
pub const COLOR_FRACT_BITS: u32 = 12;
pub const W_INVERSE_FRACT_BITS: u32 = 30;
pub const Z_FRACT_BITS: u32 = 30; // Must be greater than 16
pub const ST_FRACT_BITS: u32 = 24;
pub const ST_FILTER_FRACT_BITS: u32 = 4; // Must be less than ST_FRACT_BITS
pub const RESTORED_W_FRACT_BITS: u32 = 8; // Must be less than W_INVERSE_FRACT_BITS and ST_FRACT_BITS

pub const REG_BUS_ADDR_BIT_WIDTH: u32 = 6;

pub const REG_STATUS_ADDR: u32 = 0;
pub const REG_START_ADDR: u32 = 0;

pub const REG_TEX_CACHE_INVALIDATE_ADDR: u32 = 1;

pub const REG_DEPTH_SETTINGS_ADDR: u32 = 2;
pub const REG_DEPTH_SETTINGS_BITS: u32 = 2;
pub const REG_DEPTH_TEST_ENABLE_BIT: u32 = 0;
pub const REG_DEPTH_WRITE_MASK_ENABLE_BIT: u32 = 1;

pub const REG_TEXTURE_SETTINGS_ADDR: u32 = 3;
pub const REG_TEXTURE_SETTINGS_BITS: u32 = 3;
pub const REG_TEXTURE_SETTINGS_FILTER_SELECT_BIT_OFFSET: u32 = 0;
pub const REG_TEXTURE_SETTINGS_FILTER_SELECT_BITS: u32 = 1;
pub const REG_TEXTURE_SETTINGS_FILTER_SELECT_NEAREST: u32 = 0;
pub const REG_TEXTURE_SETTINGS_FILTER_SELECT_BILINEAR: u32 = 1;
pub const REG_TEXTURE_SETTINGS_DIM_BIT_OFFSET: u32 = REG_TEXTURE_SETTINGS_FILTER_SELECT_BIT_OFFSET + REG_TEXTURE_SETTINGS_FILTER_SELECT_BITS;
pub const REG_TEXTURE_SETTINGS_DIM_BITS: u32 = 2;
pub const REG_TEXTURE_SETTINGS_DIM_16: u32 = 0;
pub const REG_TEXTURE_SETTINGS_DIM_32: u32 = 1;
pub const REG_TEXTURE_SETTINGS_DIM_64: u32 = 2;
pub const REG_TEXTURE_SETTINGS_DIM_128: u32 = 3;

pub const REG_TEXTURE_BASE_ADDR: u32 = 4;

pub const REG_BLEND_SETTINGS_ADDR: u32 = 5;
pub const REG_BLEND_SETTINGS_BITS: u32 = 4;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_BIT_OFFSET: u32 = 0;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_BITS: u32 = 2;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_ZERO: u32 = 0;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_ONE: u32 = 1;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_SRC_ALPHA: u32 = 2;
pub const REG_BLEND_SETTINGS_SRC_FACTOR_ONE_MINUS_SRC_ALPHA: u32 = 3;
pub const REG_BLEND_SETTINGS_DST_FACTOR_BIT_OFFSET: u32 = REG_BLEND_SETTINGS_SRC_FACTOR_BIT_OFFSET + REG_BLEND_SETTINGS_SRC_FACTOR_BITS;
pub const REG_BLEND_SETTINGS_DST_FACTOR_BITS: u32 = 2;
pub const REG_BLEND_SETTINGS_DST_FACTOR_ZERO: u32 = 0;
pub const REG_BLEND_SETTINGS_DST_FACTOR_ONE: u32 = 1;
pub const REG_BLEND_SETTINGS_DST_FACTOR_SRC_ALPHA: u32 = 2;
pub const REG_BLEND_SETTINGS_DST_FACTOR_ONE_MINUS_SRC_ALPHA: u32 = 3;

pub const REG_W0_MIN_ADDR: u32 = 6;
pub const REG_W0_DX_ADDR: u32 = 7;
pub const REG_W0_DY_ADDR: u32 = 8;
pub const REG_W1_MIN_ADDR: u32 = 9;
pub const REG_W1_DX_ADDR: u32 = 10;
pub const REG_W1_DY_ADDR: u32 = 11;
pub const REG_W2_MIN_ADDR: u32 = 12;
pub const REG_W2_DX_ADDR: u32 = 13;
pub const REG_W2_DY_ADDR: u32 = 14;
pub const REG_R_MIN_ADDR: u32 = 15;
pub const REG_R_DX_ADDR: u32 = 16;
pub const REG_R_DY_ADDR: u32 = 17;
pub const REG_G_MIN_ADDR: u32 = 18;
pub const REG_G_DX_ADDR: u32 = 19;
pub const REG_G_DY_ADDR: u32 = 20;
pub const REG_B_MIN_ADDR: u32 = 21;
pub const REG_B_DX_ADDR: u32 = 22;
pub const REG_B_DY_ADDR: u32 = 23;
pub const REG_A_MIN_ADDR: u32 = 24;
pub const REG_A_DX_ADDR: u32 = 25;
pub const REG_A_DY_ADDR: u32 = 26;
pub const REG_W_INVERSE_MIN_ADDR: u32 = 27;
pub const REG_W_INVERSE_DX_ADDR: u32 = 28;
pub const REG_W_INVERSE_DY_ADDR: u32 = 29;
pub const REG_Z_MIN_ADDR: u32 = 30;
pub const REG_Z_DX_ADDR: u32 = 31;
pub const REG_Z_DY_ADDR: u32 = 32;
pub const REG_S_MIN_ADDR: u32 = 33;
pub const REG_S_DX_ADDR: u32 = 34;
pub const REG_S_DY_ADDR: u32 = 35;
pub const REG_T_MIN_ADDR: u32 = 36;
pub const REG_T_DX_ADDR: u32 = 37;
pub const REG_T_DY_ADDR: u32 = 38;

pub struct ColorThrust<'a> {
    pub m: &'a Module<'a>,

    pub reg_primary_port: PrimaryPort<'a>,
    pub color_buffer_primary_port: PrimaryPort<'a>,
    pub depth_buffer_primary_port: PrimaryPort<'a>,
    pub replica_port: ReplicaPort<'a>,
}

impl<'a> ColorThrust<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> ColorThrust<'a> {
        let m = p.module(instance_name, "ColorThrust");

        let reg_bus_ready = m.output("reg_bus_ready", m.high());
        let reg_bus_enable = m.input("reg_bus_enable", 1);
        let reg_bus_addr = m.input("reg_bus_addr", REG_BUS_ADDR_BIT_WIDTH);
        let reg_bus_write = m.input("reg_bus_write", 1);
        let reg_bus_write_data = m.input("reg_bus_write_data", 32);

        let reg_bus_write_enable = reg_bus_enable & reg_bus_write;

        let tex_cache_invalidate = reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_TEX_CACHE_INVALIDATE_ADDR, REG_BUS_ADDR_BIT_WIDTH));

        let reg_depth_settings = m.reg("depth_settings", REG_DEPTH_SETTINGS_BITS);
        reg_depth_settings.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_DEPTH_SETTINGS_ADDR, REG_BUS_ADDR_BIT_WIDTH)), {
            reg_bus_write_data.bits(REG_DEPTH_SETTINGS_BITS - 1, 0)
        }).else_({
            reg_depth_settings
        }));
        let depth_test_enable = reg_depth_settings.bit(REG_DEPTH_TEST_ENABLE_BIT);
        let depth_write_mask_enable = reg_depth_settings.bit(REG_DEPTH_WRITE_MASK_ENABLE_BIT);

        let reg_texture_settings = m.reg("texture_settings", REG_TEXTURE_SETTINGS_BITS);
        reg_texture_settings.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_TEXTURE_SETTINGS_ADDR, REG_BUS_ADDR_BIT_WIDTH)), {
            reg_bus_write_data.bits(REG_TEXTURE_SETTINGS_BITS - 1, 0)
        }).else_({
            reg_texture_settings
        }));
        let tex_filter_select = reg_texture_settings.bit(REG_TEXTURE_SETTINGS_FILTER_SELECT_BIT_OFFSET);
        let tex_dim = reg_texture_settings.bits(REG_TEXTURE_SETTINGS_DIM_BIT_OFFSET + REG_TEXTURE_SETTINGS_DIM_BITS - 1, REG_TEXTURE_SETTINGS_DIM_BIT_OFFSET);

        let reg_texture_base = m.reg("texture_base", TEX_PIXEL_ADDR_BITS - 8);
        reg_texture_base.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_TEXTURE_BASE_ADDR, REG_BUS_ADDR_BIT_WIDTH)), {
            // TODO: I don't think this bit range is correct!
            reg_bus_write_data.bits(TEX_PIXEL_ADDR_BITS - 1, 8)
        }).else_({
            reg_texture_base
        }));

        let reg_blend_settings = m.reg("blend_settings", REG_BLEND_SETTINGS_BITS);
        reg_blend_settings.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_BLEND_SETTINGS_ADDR, REG_BUS_ADDR_BIT_WIDTH)), {
            reg_bus_write_data.bits(REG_BLEND_SETTINGS_BITS - 1, 0)
        }).else_({
            reg_blend_settings
        }));
        let blend_src_factor = reg_blend_settings.bits(REG_BLEND_SETTINGS_SRC_FACTOR_BIT_OFFSET + REG_BLEND_SETTINGS_SRC_FACTOR_BITS - 1, REG_BLEND_SETTINGS_SRC_FACTOR_BIT_OFFSET);
        let blend_dst_factor = reg_blend_settings.bits(REG_BLEND_SETTINGS_DST_FACTOR_BIT_OFFSET + REG_BLEND_SETTINGS_DST_FACTOR_BITS - 1, REG_BLEND_SETTINGS_DST_FACTOR_BIT_OFFSET);

        let input_generator_active = m.reg("input_generator_active", 1);
        input_generator_active.default_value(false);

        let tile_x = m.reg("tile_x", TILE_DIM_BITS);
        let tile_y = m.reg("tile_y", TILE_DIM_BITS);
        let tile_x_last = tile_x.eq(m.lit(TILE_DIM - 1, TILE_DIM_BITS));
        let tile_y_last = tile_y.eq(m.lit(TILE_DIM - 1, TILE_DIM_BITS));

        let start = reg_bus_write_enable & reg_bus_addr.eq(m.lit(REG_START_ADDR, REG_BUS_ADDR_BIT_WIDTH));

        generate_pixel_pipe(c);
        let pixel_pipe = m.instance("pixel_pipe", "PixelPipe");

        pixel_pipe.start.drive(start);

        pixel_pipe.depth_test_enable.drive(depth_test_enable);
        pixel_pipe.depth_write_mask_enable.drive(depth_write_mask_enable);

        pixel_pipe.tex_filter_select.drive(tex_filter_select);
        pixel_pipe.tex_dim.drive(tex_dim);
        pixel_pipe.tex_base.drive(reg_texture_base);

        pixel_pipe.blend_src_factor.drive(blend_src_factor);
        pixel_pipe.blend_dst_factor.drive(blend_dst_factor);

        pixel_pipe.in_valid.drive(input_generator_active);
        pixel_pipe.in_tile_addr.drive(tile_y.concat(tile_x));

        let pixel_pipe_in_ready = pixel_pipe.in_ready;

        let (next_input_generator_active, next_tile_x, next_tile_y) = if_(start, {
            let next_input_generator_active = m.high();

            let next_tile_x = m.lit(0u32, TILE_DIM_BITS);
            let next_tile_y = m.lit(0u32, TILE_DIM_BITS);

            (next_input_generator_active, next_tile_x, next_tile_y)
        }).else_if(pixel_pipe_in_ready, {
            let next_input_generator_active = input_generator_active;

            let next_tile_x = tile_x + m.lit(1u32, TILE_DIM_BITS);
            let next_tile_y = tile_y;

            let (next_input_generator_active, next_tile_y) = if_(tile_x_last, {
                let next_input_generator_active = if_(tile_y_last, {
                    m.low()
                }).else_({
                    next_input_generator_active
                });

                let next_tile_y = tile_y + m.lit(1u32, TILE_DIM_BITS);

                (next_input_generator_active, next_tile_y)
            }).else_({
                (next_input_generator_active, next_tile_y)
            });

            (next_input_generator_active, next_tile_x, next_tile_y)
        }).else_({
            (input_generator_active, tile_x, tile_y)
        });

        input_generator_active.drive_next(next_input_generator_active);

        tile_x.drive_next(next_tile_x);
        tile_y.drive_next(next_tile_y);

        let interpolant = |name: &str, num_bits, min_addr: u32, dx_addr: u32, dy_addr: u32| {
            let min = m.reg(format!("{}_min", name), num_bits);
            min.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(min_addr, REG_BUS_ADDR_BIT_WIDTH)), {
                reg_bus_write_data.bits(num_bits - 1, 0)
            }).else_({
                min
            }));
            let dx = m.reg(format!("{}_dx", name), num_bits);
            dx.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(dx_addr, REG_BUS_ADDR_BIT_WIDTH)), {
                reg_bus_write_data.bits(num_bits - 1, 0)
            }).else_({
                dx
            }));
            let dx_mirror = m.reg(format!("{}_dx_mirror", name), num_bits);
            dx_mirror.drive_next(if_(start, {
                dx
            }).else_({
                dx_mirror
            }));
            let dy = m.reg(format!("{}_dy", name), num_bits);
            dy.drive_next(if_(reg_bus_write_enable & reg_bus_addr.eq(m.lit(dy_addr, REG_BUS_ADDR_BIT_WIDTH)), {
                reg_bus_write_data.bits(num_bits - 1, 0)
            }).else_({
                dy
            }));
            let dy_mirror = m.reg(format!("{}_dy_mirror", name), num_bits);
            dy_mirror.drive_next(if_(start, {
                dy
            }).else_({
                dy_mirror
            }));

            let row = m.reg(format!("{}_row", name), num_bits);

            let value = m.reg(name, num_bits);

            let (next_row, next_value) = if_(start, {
                (min, min)
            }).else_if(pixel_pipe_in_ready, {
                if_(tile_x_last, {
                    let next = row + dy_mirror;
                    (next, next)
                }).else_({
                    (row, value + dx_mirror)
                })
            }).else_({
                (row, value)
            });

            row.drive_next(next_row);

            value.drive_next(next_value);

            value
        };

        let w0 = interpolant("w0", 32, REG_W0_MIN_ADDR, REG_W0_DX_ADDR, REG_W0_DY_ADDR).bit(31);
        let w1 = interpolant("w1", 32, REG_W1_MIN_ADDR, REG_W1_DX_ADDR, REG_W1_DY_ADDR).bit(31);
        let w2 = interpolant("w2", 32, REG_W2_MIN_ADDR, REG_W2_DX_ADDR, REG_W2_DY_ADDR).bit(31);

        let r = interpolant("r", 24, REG_R_MIN_ADDR, REG_R_DX_ADDR, REG_R_DY_ADDR).bits(COLOR_WHOLE_BITS + COLOR_FRACT_BITS - 1, COLOR_FRACT_BITS);
        let g = interpolant("g", 24, REG_G_MIN_ADDR, REG_G_DX_ADDR, REG_G_DY_ADDR).bits(COLOR_WHOLE_BITS + COLOR_FRACT_BITS - 1, COLOR_FRACT_BITS);
        let b = interpolant("b", 24, REG_B_MIN_ADDR, REG_B_DX_ADDR, REG_B_DY_ADDR).bits(COLOR_WHOLE_BITS + COLOR_FRACT_BITS - 1, COLOR_FRACT_BITS);
        let a = interpolant("a", 24, REG_A_MIN_ADDR, REG_A_DX_ADDR, REG_A_DY_ADDR).bits(COLOR_WHOLE_BITS + COLOR_FRACT_BITS - 1, COLOR_FRACT_BITS);

        let w_inverse = interpolant("w_inverse", 32, REG_W_INVERSE_MIN_ADDR, REG_W_INVERSE_DX_ADDR, REG_W_INVERSE_DY_ADDR);

        let z = interpolant("z", 32, REG_Z_MIN_ADDR, REG_Z_DX_ADDR, REG_Z_DY_ADDR).bits(31, 16);

        let s = interpolant("s", 32, REG_S_MIN_ADDR, REG_S_DX_ADDR, REG_S_DY_ADDR).bits(31, RESTORED_W_FRACT_BITS);
        let t = interpolant("t", 32, REG_T_MIN_ADDR, REG_T_DX_ADDR, REG_T_DY_ADDR).bits(31, RESTORED_W_FRACT_BITS);

        pixel_pipe.in_w0.drive(w0);
        pixel_pipe.in_w1.drive(w1);
        pixel_pipe.in_w2.drive(w2);

        pixel_pipe.in_r.drive(r);
        pixel_pipe.in_g.drive(g);
        pixel_pipe.in_b.drive(b);
        pixel_pipe.in_a.drive(a);

        pixel_pipe.in_w_inverse.drive(w_inverse);

        pixel_pipe.in_z.drive(z);

        pixel_pipe.in_s.drive(s);
        pixel_pipe.in_t.drive(t);

        let reg_bus_read_data = m.output("reg_bus_read_data", m.lit(0u32, 31).concat(input_generator_active | pixel_pipe.output("active")));
        let reg_bus_read_data_valid = m.output("reg_bus_read_data_valid", (reg_bus_enable & !reg_bus_write).reg_next_with_default("reg_bus_read_data_valid", false));

        let color_buffer_bus_ready = m.output("color_buffer_bus_ready", m.high());
        let color_buffer_bus_enable = m.input("color_buffer_bus_enable", 1);
        let color_buffer_bus_addr = m.input("color_buffer_bus_addr", TILE_PIXELS_WORDS_BITS);
        let color_buffer_bus_write = m.input("color_buffer_bus_write", 1);
        let color_buffer_bus_write_data = m.input("color_buffer_bus_write_data", 128);
        let color_buffer_bus_write_byte_enable = m.input("color_buffer_bus_write_byte_enable", 16);
        let color_buffer_bus_write_word_enable = (0..4).fold(None, |acc, x| {
            let word_enable_bit = color_buffer_bus_write_byte_enable.bit(x * 4);
            Some(if let Some(acc) = acc {
                word_enable_bit.concat(acc)
            } else {
                word_enable_bit
            })
        }).unwrap();

        let color_buffer = WordMem::new(m, "color_buffer", TILE_PIXELS_WORDS_BITS, 32, 4);
        let color_buffer_bus_write_enable = color_buffer_bus_enable & color_buffer_bus_write;
        color_buffer.write_port(
            if_(color_buffer_bus_write_enable, {
                color_buffer_bus_addr
            }).else_({
                pixel_pipe.color_buffer_write_port_addr
            }),
            if_(color_buffer_bus_write_enable, {
                color_buffer_bus_write_data
            }).else_({
                pixel_pipe.color_buffer_write_port_value
            }),
            color_buffer_bus_write_enable | pixel_pipe.color_buffer_write_port_enable,
            if_(color_buffer_bus_write_enable, {
                color_buffer_bus_write_word_enable
            }).else_({
                pixel_pipe.color_buffer_write_port_word_enable
            }));

        let color_buffer_bus_read_enable = color_buffer_bus_enable & !color_buffer_bus_write;
        let color_buffer_read_port_value = color_buffer.read_port(
            if_(color_buffer_bus_read_enable, {
                color_buffer_bus_addr
            }).else_({
                pixel_pipe.color_buffer_read_port_addr
            }),
            color_buffer_bus_read_enable | pixel_pipe.color_buffer_read_port_enable);

        pixel_pipe.drive_input("color_buffer_read_port_value", color_buffer_read_port_value);

        let color_buffer_read_port_value = m.output("color_buffer_bus_read_data", color_buffer_read_port_value);
        let color_buffer_bus_read_data_valid = m.output("color_buffer_bus_read_data_valid", color_buffer_bus_read_enable.reg_next_with_default("color_buffer_bus_read_data_valid", false));

        let depth_buffer_bus_ready = m.output("depth_buffer_bus_ready", m.high());
        let depth_buffer_bus_enable = m.input("depth_buffer_bus_enable", 1);
        let depth_buffer_bus_addr = m.input("depth_buffer_bus_addr", TILE_PIXELS_WORDS_BITS - 1);
        let depth_buffer_bus_write = m.input("depth_buffer_bus_write", 1);
        let depth_buffer_bus_write_data = m.input("depth_buffer_bus_write_data", 128);
        let depth_buffer_bus_write_byte_enable = m.input("depth_buffer_bus_write_byte_enable", 16);
        let depth_buffer_bus_write_word_enable = (0..8).fold(None, |acc, x| {
            let word_enable_bit = depth_buffer_bus_write_byte_enable.bit(x * 2);
            Some(if let Some(acc) = acc {
                word_enable_bit.concat(acc)
            } else {
                word_enable_bit
            })
        }).unwrap();

        let depth_buffer = WordMem::new(m, "depth_buffer", TILE_PIXELS_WORDS_BITS - 1, 16, 8);
        let depth_buffer_bus_write_enable = depth_buffer_bus_enable & depth_buffer_bus_write;
        depth_buffer.write_port(
            if_(depth_buffer_bus_write_enable, {
                depth_buffer_bus_addr
            }).else_({
                pixel_pipe.depth_buffer_write_port_addr
            }),
            if_(depth_buffer_bus_write_enable, {
                depth_buffer_bus_write_data
            }).else_({
                pixel_pipe.depth_buffer_write_port_value
            }),
            depth_buffer_bus_write_enable | pixel_pipe.depth_buffer_write_port_enable,
            if_(depth_buffer_bus_write_enable, {
                depth_buffer_bus_write_word_enable
            }).else_({
                pixel_pipe.depth_buffer_write_port_word_enable
            }));

        let depth_buffer_bus_read_enable = depth_buffer_bus_enable & !depth_buffer_bus_write;
        let depth_buffer_read_port_value = depth_buffer.read_port(
            if_(depth_buffer_bus_read_enable, {
                depth_buffer_bus_addr
            }).else_({
                pixel_pipe.depth_buffer_read_port_addr
            }),
            depth_buffer_bus_read_enable | pixel_pipe.depth_buffer_read_port_enable);

        pixel_pipe.depth_buffer_read_port_value.drive(depth_buffer_read_port_value);

        let depth_buffer_bus_read_data = m.output("depth_buffer_bus_read_data", depth_buffer_read_port_value);
        let depth_buffer_bus_read_data_valid = m.output("depth_buffer_bus_read_data_valid", depth_buffer_bus_read_enable.reg_next_with_default("depth_buffer_bus_read_data_valid", false));

        pixel_pipe.tex_cache_invalidate.drive(tex_cache_invalidate);

        ColorThrust {
            m,

            reg_primary_port: PrimaryPort {
                bus_enable: reg_bus_enable,
                bus_addr: reg_bus_addr,
                bus_write: reg_bus_write,
                bus_write_data: reg_bus_write_data,
                bus_write_byte_enable: reg_bus_write_byte_enable,
                bus_ready: reg_bus_ready,
                bus_read_data: reg_bus_read_data,
                bus_read_data_valid: reg_bus_read_data_valid,
            },
            color_buffer_primary_port: PrimaryPort {
                bus_enable: color_buffer_bus_enable,
                bus_addr: color_buffer_bus_addr,
                bus_write: color_buffer_bus_write,
                bus_write_data: color_buffer_bus_write_data,
                bus_write_byte_enable: color_buffer_bus_write_byte_enable,
                bus_ready: color_buffer_bus_ready,
                bus_read_data: color_buffer_bus_read_data,
                bus_read_data_valid: color_buffer_bus_read_data_valid,
            },
            depth_buffer_primary_port: PrimaryPort {
                bus_enable: depth_buffer_bus_enable,
                bus_addr: depth_buffer_bus_addr,
                bus_write: depth_buffer_bus_write,
                bus_write_data: depth_buffer_bus_write_data,
                bus_write_byte_enable: depth_buffer_bus_write_byte_enable,
                bus_ready: depth_buffer_bus_ready,
                bus_read_data: depth_buffer_bus_read_data,
                bus_read_data_valid: depth_buffer_bus_read_data_valid,
            },
            replica_port: pixel_pipe.replica_port.forward("replica", m),
        }
    }
}

pub fn generate_pixel_pipe<'a>(c: &'a Context<'a>) -> &Module<'a> {
    let m = c.module("PixelPipe");

    // Control
    let start = m.input("start", 1);

    let active = m.reg("active", 1);
    active.default_value(false);
    m.output("active", active);

    let finished_pixel_acc = m.reg("finished_pixel_acc", TILE_PIXELS_BITS + 1);

    // Inputs
    let valid = m.input("in_valid", 1);
    let tile_addr = m.input("in_tile_addr", TILE_PIXELS_BITS);

    //  Reject pixel if it doesn't pass edge test before it enters the first pipe
    let w0 = m.input("in_w0", 1);
    let w1 = m.input("in_w1", 1);
    let w2 = m.input("in_w2", 1);
    let edge_test = !(w0 | w1 | w2);
    let edge_test_reject = valid & !edge_test;
    let valid = valid & edge_test;

    let r = m.input("in_r", COLOR_WHOLE_BITS);
    let g = m.input("in_g", COLOR_WHOLE_BITS);
    let b = m.input("in_b", COLOR_WHOLE_BITS);
    let a = m.input("in_a", COLOR_WHOLE_BITS);

    let w_inverse = m.input("in_w_inverse", 32);

    let z = m.input("in_z", 16);

    let s = m.input("in_s", 32 - RESTORED_W_FRACT_BITS);
    let t = m.input("in_t", 32 - RESTORED_W_FRACT_BITS);

    // Depth test pipe
    generate_depth_test_pipe(c);
    let mut depth_test_pipe = flow_controlled_pipe::FlowControlledPipe::new(
        c,
        "FlowControlledDepthTestPipe",
        "DepthTestPipe",
        2);

    //  Aux
    depth_test_pipe.aux_input("depth_test_enable", 1);

    depth_test_pipe.aux_output("depth_buffer_read_port_addr");
    depth_test_pipe.aux_output("depth_buffer_read_port_enable");

    depth_test_pipe.aux_input("depth_buffer_read_port_value", 128);

    //  Inputs
    depth_test_pipe.input("tile_addr", TILE_PIXELS_BITS);

    depth_test_pipe.input("r", COLOR_WHOLE_BITS);
    depth_test_pipe.input("g", COLOR_WHOLE_BITS);
    depth_test_pipe.input("b", COLOR_WHOLE_BITS);
    depth_test_pipe.input("a", COLOR_WHOLE_BITS);

    depth_test_pipe.input("w_inverse", 32);

    depth_test_pipe.input("z", 16);

    depth_test_pipe.input("s", 32 - RESTORED_W_FRACT_BITS);
    depth_test_pipe.input("t", 32 - RESTORED_W_FRACT_BITS);

    //  Outputs
    depth_test_pipe.output("tile_addr", TILE_PIXELS_BITS);

    depth_test_pipe.output("r", COLOR_WHOLE_BITS);
    depth_test_pipe.output("g", COLOR_WHOLE_BITS);
    depth_test_pipe.output("b", COLOR_WHOLE_BITS);
    depth_test_pipe.output("a", COLOR_WHOLE_BITS);

    depth_test_pipe.output("w_inverse", 32);

    depth_test_pipe.output("z", 16);

    depth_test_pipe.output("s", 32 - RESTORED_W_FRACT_BITS);
    depth_test_pipe.output("t", 32 - RESTORED_W_FRACT_BITS);

    depth_test_pipe.output("depth_test_result", 1);

    let depth_test_pipe = m.instance("depth_test_pipe", "FlowControlledDepthTestPipe");

    m.output("in_ready", depth_test_pipe.output("in_ready") | edge_test_reject);

    //  Aux
    depth_test_pipe.drive_input("depth_test_enable", m.input("depth_test_enable", 1));

    m.output("depth_buffer_read_port_addr", depth_test_pipe.output("depth_buffer_read_port_addr"));
    m.output("depth_buffer_read_port_enable", depth_test_pipe.output("depth_buffer_read_port_enable"));

    depth_test_pipe.drive_input("depth_buffer_read_port_value", m.input("depth_buffer_read_port_value", 128));

    //  Inputs
    depth_test_pipe.drive_input("in_valid", valid);
    depth_test_pipe.drive_input("in_tile_addr", tile_addr);

    depth_test_pipe.drive_input("in_r", r);
    depth_test_pipe.drive_input("in_g", g);
    depth_test_pipe.drive_input("in_b", b);
    depth_test_pipe.drive_input("in_a", a);

    depth_test_pipe.drive_input("in_w_inverse", w_inverse);

    depth_test_pipe.drive_input("in_z", z);

    depth_test_pipe.drive_input("in_s", s);
    depth_test_pipe.drive_input("in_t", t);

    //  Outputs
    let valid = depth_test_pipe.output("out_valid");
    let tile_addr = depth_test_pipe.output("out_tile_addr");

    let r = depth_test_pipe.output("out_r");
    let g = depth_test_pipe.output("out_g");
    let b = depth_test_pipe.output("out_b");
    let a = depth_test_pipe.output("out_a");

    let w_inverse = depth_test_pipe.output("out_w_inverse");

    let z = depth_test_pipe.output("out_z");

    let s = depth_test_pipe.output("out_s");
    let t = depth_test_pipe.output("out_t");

    let depth_test_result = depth_test_pipe.output("out_depth_test_result");

    // Reject pixel if it doesn't pass depth test before entering the next pipe
    let depth_test_reject = valid & !depth_test_result;
    let valid = valid & depth_test_result;

    // Front pipe
    generate_front_pipe(c);
    let mut front_pipe = flow_controlled_pipe::FlowControlledPipe::new(
        c,
        "FlowControlledFrontPipe",
        "FrontPipe",
        15);

    //  Aux
    front_pipe.aux_input("tex_filter_select", 1);
    front_pipe.aux_input("tex_dim", 2);
    front_pipe.aux_input("tex_base", TEX_PIXEL_ADDR_BITS - 8);

    //  Inputs
    front_pipe.input("tile_addr", TILE_PIXELS_BITS);

    front_pipe.input("r", COLOR_WHOLE_BITS);
    front_pipe.input("g", COLOR_WHOLE_BITS);
    front_pipe.input("b", COLOR_WHOLE_BITS);
    front_pipe.input("a", COLOR_WHOLE_BITS);

    front_pipe.input("w_inverse", 32);

    front_pipe.input("z", 16);

    front_pipe.input("s", 32 - RESTORED_W_FRACT_BITS);
    front_pipe.input("t", 32 - RESTORED_W_FRACT_BITS);

    front_pipe.input("depth_test_result", 1);

    //  Outputs
    front_pipe.output("tile_addr", TILE_PIXELS_BITS);

    front_pipe.output("r", 9);
    front_pipe.output("g", 9);
    front_pipe.output("b", 9);
    front_pipe.output("a", 9);

    front_pipe.output("z", 16);

    front_pipe.output("depth_test_result", 1);

    front_pipe.output("s_fract", ST_FILTER_FRACT_BITS + 1);
    front_pipe.output("one_minus_s_fract", ST_FILTER_FRACT_BITS + 1);
    front_pipe.output("t_fract", ST_FILTER_FRACT_BITS + 1);
    front_pipe.output("one_minus_t_fract", ST_FILTER_FRACT_BITS + 1);

    front_pipe.output("tex_buffer0_read_addr", TEX_PIXEL_ADDR_BITS);
    front_pipe.output("tex_buffer1_read_addr", TEX_PIXEL_ADDR_BITS);
    front_pipe.output("tex_buffer2_read_addr", TEX_PIXEL_ADDR_BITS);
    front_pipe.output("tex_buffer3_read_addr", TEX_PIXEL_ADDR_BITS);

    let front_pipe = m.instance("front_pipe", "FlowControlledFrontPipe");

    depth_test_pipe.drive_input("out_ready", front_pipe.output("in_ready") | depth_test_reject);

    //  Aux
    front_pipe.drive_input("tex_filter_select", m.input("tex_filter_select", 1));
    front_pipe.drive_input("tex_dim", m.input("tex_dim", 2));
    front_pipe.drive_input("tex_base", m.input("tex_base", TEX_PIXEL_ADDR_BITS - 8));

    //  Inputs
    front_pipe.drive_input("in_valid", valid);
    front_pipe.drive_input("in_tile_addr", tile_addr);

    front_pipe.drive_input("in_r", r);
    front_pipe.drive_input("in_g", g);
    front_pipe.drive_input("in_b", b);
    front_pipe.drive_input("in_a", a);

    front_pipe.drive_input("in_w_inverse", w_inverse);

    front_pipe.drive_input("in_z", z);

    front_pipe.drive_input("in_s", s);
    front_pipe.drive_input("in_t", t);

    front_pipe.drive_input("in_depth_test_result", depth_test_result);

    //  Outputs
    let valid = front_pipe.output("out_valid");
    let tile_addr = front_pipe.output("out_tile_addr");

    let r = front_pipe.output("out_r");
    let g = front_pipe.output("out_g");
    let b = front_pipe.output("out_b");
    let a = front_pipe.output("out_a");

    let z = front_pipe.output("out_z");

    let depth_test_result = front_pipe.output("out_depth_test_result");

    let s_fract = front_pipe.output("out_s_fract");
    let one_minus_s_fract = front_pipe.output("out_one_minus_s_fract");
    let t_fract = front_pipe.output("out_t_fract");
    let one_minus_t_fract = front_pipe.output("out_one_minus_t_fract");

    // Tex cache
    tex_cache::generate(c);
    let tex_cache = m.instance("tex_cache", "TexCache");

    //  Aux
    tex_cache.drive_input("invalidate", m.input("tex_cache_invalidate", 1));

    tex_cache.drive_input("replica_bus_ready", m.input("replica_bus_ready", 1));
    m.output("replica_bus_enable", tex_cache.output("replica_bus_enable"));
    m.output("replica_bus_addr", tex_cache.output("replica_bus_addr"));
    tex_cache.drive_input("replica_bus_read_data", m.input("replica_bus_read_data", 128));
    tex_cache.drive_input("replica_bus_read_data_valid", m.input("replica_bus_read_data_valid", 1));

    //  Inputs
    front_pipe.drive_input("out_ready", tex_cache.output("in_ready"));

    tex_cache.drive_input("in_valid", valid);
    tex_cache.drive_input("in_tile_addr", tile_addr);

    tex_cache.drive_input("in_r", r);
    tex_cache.drive_input("in_g", g);
    tex_cache.drive_input("in_b", b);
    tex_cache.drive_input("in_a", a);

    tex_cache.drive_input("in_z", z);

    tex_cache.drive_input("in_depth_test_result", depth_test_result);

    tex_cache.drive_input("in_s_fract", s_fract);
    tex_cache.drive_input("in_one_minus_s_fract", one_minus_s_fract);
    tex_cache.drive_input("in_t_fract", t_fract);
    tex_cache.drive_input("in_one_minus_t_fract", one_minus_t_fract);

    for i in 0..4 {
        tex_cache.drive_input(format!("in_tex_buffer{}_read_addr", i), front_pipe.output(format!("out_tex_buffer{}_read_addr", i)));
    }

    //  Outputs
    let valid = tex_cache.output("out_valid");
    let tile_addr = tex_cache.output("out_tile_addr");

    let r = tex_cache.output("out_r");
    let g = tex_cache.output("out_g");
    let b = tex_cache.output("out_b");
    let a = tex_cache.output("out_a");

    let z = tex_cache.output("out_z");

    let depth_test_result = tex_cache.output("out_depth_test_result");

    let s_fract = tex_cache.output("out_s_fract");
    let one_minus_s_fract = tex_cache.output("out_one_minus_s_fract");
    let t_fract = tex_cache.output("out_t_fract");
    let one_minus_t_fract = tex_cache.output("out_one_minus_t_fract");

    // Back pipe
    generate_back_pipe(c);
    let back_pipe = m.instance("back_pipe", "BackPipe");

    //  Aux
    back_pipe.drive_input("depth_write_mask_enable", m.input("depth_write_mask_enable", 1));

    back_pipe.drive_input("blend_src_factor", m.input("blend_src_factor", REG_BLEND_SETTINGS_SRC_FACTOR_BITS));
    back_pipe.drive_input("blend_dst_factor", m.input("blend_dst_factor", REG_BLEND_SETTINGS_DST_FACTOR_BITS));

    m.output("color_buffer_read_port_addr", back_pipe.output("color_buffer_read_port_addr"));
    m.output("color_buffer_read_port_enable", back_pipe.output("color_buffer_read_port_enable"));

    back_pipe.drive_input("color_buffer_read_port_value", m.input("color_buffer_read_port_value", 128));

    m.output("color_buffer_write_port_addr", back_pipe.output("color_buffer_write_port_addr"));
    m.output("color_buffer_write_port_value", back_pipe.output("color_buffer_write_port_value"));
    m.output("color_buffer_write_port_enable", back_pipe.output("color_buffer_write_port_enable"));
    m.output("color_buffer_write_port_word_enable", back_pipe.output("color_buffer_write_port_word_enable"));

    m.output("depth_buffer_write_port_addr", back_pipe.output("depth_buffer_write_port_addr"));
    m.output("depth_buffer_write_port_value", back_pipe.output("depth_buffer_write_port_value"));
    m.output("depth_buffer_write_port_enable", back_pipe.output("depth_buffer_write_port_enable"));
    m.output("depth_buffer_write_port_word_enable", back_pipe.output("depth_buffer_write_port_word_enable"));

    //  Inputs
    back_pipe.drive_input("in_valid", valid);
    back_pipe.drive_input("in_tile_addr", tile_addr);

    back_pipe.drive_input("in_r", r);
    back_pipe.drive_input("in_g", g);
    back_pipe.drive_input("in_b", b);
    back_pipe.drive_input("in_a", a);

    back_pipe.drive_input("in_z", z);

    back_pipe.drive_input("in_depth_test_result", depth_test_result);

    back_pipe.drive_input("in_s_fract", s_fract);
    back_pipe.drive_input("in_one_minus_s_fract", one_minus_s_fract);
    back_pipe.drive_input("in_t_fract", t_fract);
    back_pipe.drive_input("in_one_minus_t_fract", one_minus_t_fract);

    for i in 0..4 {
        back_pipe.drive_input(format!("in_tex_buffer{}_read_value", i), tex_cache.output(format!("out_tex_buffer{}_read_value", i)));
    }

    //  Outputs
    let valid = back_pipe.output("out_valid");

    active.drive_next(if_(start, {
        m.high()
    }).else_if(finished_pixel_acc.eq(m.lit(TILE_PIXELS, TILE_PIXELS_BITS + 1)), {
        m.low()
    }).else_({
        active
    }));

    // TODO: There's gotta be a simpler way to do this!
    let finished_pixel_bits = edge_test_reject.concat(depth_test_reject).concat(valid);
    let finished_pixel_count = if_(finished_pixel_bits.eq(m.lit(0b000u32, 3)), {
        m.lit(0u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b001u32, 3)), {
        m.lit(1u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b010u32, 3)), {
        m.lit(1u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b011u32, 3)), {
        m.lit(2u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b100u32, 3)), {
        m.lit(1u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b101u32, 3)), {
        m.lit(2u32, 2)
    }).else_if(finished_pixel_bits.eq(m.lit(0b110u32, 3)), {
        m.lit(2u32, 2)
    }).else_({
        m.lit(3u32, 2)
    });

    finished_pixel_acc.drive_next(if_(start, {
        m.lit(0u32, TILE_PIXELS_BITS + 1)
    }).else_({
        finished_pixel_acc + m.lit(0u32, TILE_PIXELS_BITS - 1).concat(finished_pixel_count)
    }));

    m
}

pub fn generate_depth_test_pipe<'a>(c: &'a Context<'a>) -> &Module<'a> {
    let m = c.module("DepthTestPipe");

    // Aux inputs
    let depth_test_enable = m.input("depth_test_enable", 1);

    // Inputs
    let valid = m.input("in_valid", 1);
    let tile_addr = m.input("in_tile_addr", TILE_PIXELS_BITS);

    let r = m.input("in_r", COLOR_WHOLE_BITS);
    let g = m.input("in_g", COLOR_WHOLE_BITS);
    let b = m.input("in_b", COLOR_WHOLE_BITS);
    let a = m.input("in_a", COLOR_WHOLE_BITS);

    let w_inverse = m.input("in_w_inverse", 32);

    let z = m.input("in_z", 16);

    let s = m.input("in_s", 32 - RESTORED_W_FRACT_BITS);
    let t = m.input("in_t", 32 - RESTORED_W_FRACT_BITS);

    //  Issue depth buffer read for prev_depth
    m.output("depth_buffer_read_port_addr", tile_addr.bits(TILE_PIXELS_BITS - 1, 3));
    m.output("depth_buffer_read_port_enable", valid & depth_test_enable);

    // Stage 1
    let valid = valid.reg_next_with_default("stage_1_valid", false);
    let tile_addr = tile_addr.reg_next("stage_1_tile_addr");

    let r = r.reg_next("stage_1_r");
    let g = g.reg_next("stage_1_g");
    let b = b.reg_next("stage_1_b");
    let a = a.reg_next("stage_1_a");

    let w_inverse = w_inverse.reg_next("stage_1_w_inverse");

    let z = z.reg_next("stage_1_z");

    let s = s.reg_next("stage_1_s");
    let t = t.reg_next("stage_1_t");

    //  Returned from issue in previous stage
    let prev_depth = m.input("depth_buffer_read_port_value", 128);
    let prev_depth = if_(tile_addr.bits(2, 0).eq(m.lit(0u32, 3)), {
        prev_depth.bits(15, 0)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(1u32, 3)), {
        prev_depth.bits(31, 16)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(2u32, 3)), {
        prev_depth.bits(47, 32)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(3u32, 3)), {
        prev_depth.bits(63, 48)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(4u32, 3)), {
        prev_depth.bits(79, 64)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(5u32, 3)), {
        prev_depth.bits(95, 80)
    }).else_if(tile_addr.bits(2, 0).eq(m.lit(6u32, 3)), {
        prev_depth.bits(111, 96)
    }).else_({
        prev_depth.bits(127, 112)
    });

    // Stage 2
    let valid = valid.reg_next_with_default("stage_2_valid", false);
    let tile_addr = tile_addr.reg_next("stage_2_tile_addr");

    let r = r.reg_next("stage_2_r");
    let g = g.reg_next("stage_2_g");
    let b = b.reg_next("stage_2_b");
    let a = a.reg_next("stage_2_a");

    let w_inverse = w_inverse.reg_next("stage_2_w_inverse");

    let z = z.reg_next("stage_2_z");

    let s = s.reg_next("stage_2_s");
    let t = t.reg_next("stage_2_t");

    let prev_depth = prev_depth.reg_next("stage_2_prev_depth");

    let depth_test_result = z.lt(prev_depth) | !depth_test_enable;

    // Outputs
    m.output("out_valid", valid);
    m.output("out_tile_addr", tile_addr);

    m.output("out_r", r);
    m.output("out_g", g);
    m.output("out_b", b);
    m.output("out_a", a);

    m.output("out_w_inverse", w_inverse);

    m.output("out_z", z);

    m.output("out_s", s);
    m.output("out_t", t);

    m.output("out_depth_test_result", depth_test_result);

    m
}

pub fn generate_front_pipe<'a>(c: &'a Context<'a>) -> &Module<'a> {
    let m = c.module("FrontPipe");

    // Aux inputs
    let tex_filter_select = m.input("tex_filter_select", 1);
    let tex_dim = m.input("tex_dim", 2);
    let tex_base = m.input("tex_base", TEX_PIXEL_ADDR_BITS - 8);

    // Inputs
    let mut valid = m.input("in_valid", 1);
    let mut tile_addr = m.input("in_tile_addr", TILE_PIXELS_BITS);

    let mut r = m.input("in_r", COLOR_WHOLE_BITS);
    let mut g = m.input("in_g", COLOR_WHOLE_BITS);
    let mut b = m.input("in_b", COLOR_WHOLE_BITS);
    let mut a = m.input("in_a", COLOR_WHOLE_BITS);

    let w_inverse = m.input("in_w_inverse", 32);

    let mut z = m.input("in_z", 16);

    let mut s = m.input("in_s", 32 - RESTORED_W_FRACT_BITS);
    let mut t = m.input("in_t", 32 - RESTORED_W_FRACT_BITS);

    let mut depth_test_result = m.input("in_depth_test_result", 1);

    approx_reciprocal::generate(c, "WInverseReciprocal", W_INVERSE_FRACT_BITS - RESTORED_W_FRACT_BITS - 3, 4);
    let w_approx_reciprocal = m.instance("w_approx_reciprocal", "WInverseReciprocal");
    w_approx_reciprocal.drive_input("x", w_inverse);

    // Stages 1-13 (mostly just delay for w to arrive)
    for stage in 1..=13 {
        valid = valid.reg_next_with_default(format!("stage_{}_valid", stage), false);
        tile_addr = tile_addr.reg_next(format!("stage_{}_tile_addr", stage));

        r = r.reg_next(format!("stage_{}_r", stage));
        g = g.reg_next(format!("stage_{}_g", stage));
        b = b.reg_next(format!("stage_{}_b", stage));
        a = a.reg_next(format!("stage_{}_a", stage));

        z = z.reg_next(format!("stage_{}_z", stage));

        s = s.reg_next(format!("stage_{}_s", stage));
        t = t.reg_next(format!("stage_{}_t", stage));

        depth_test_result = depth_test_result.reg_next(format!("stage_{}_depth_test_result", stage));
    }

    //  Returned from issue before stage 1
    let w = w_approx_reciprocal.output("quotient");

    // Stage 14
    let valid = valid.reg_next_with_default("stage_14_valid", false);
    let tile_addr = tile_addr.reg_next("stage_14_tile_addr");

    let r = r.reg_next("stage_14_r");
    let g = g.reg_next("stage_14_g");
    let b = b.reg_next("stage_14_b");
    let a = a.reg_next("stage_14_a");

    let z = z.reg_next("stage_14_z");

    let s = s.reg_next("stage_14_s");
    let t = t.reg_next("stage_14_t");

    let w = w.reg_next("stage_14_w");

    let s = s.mul_signed(w);
    let t = t.mul_signed(w);

    let depth_test_result = depth_test_result.reg_next("stage_14_depth_test_result");

    // Stage 15
    let valid = valid.reg_next_with_default("stage_15_valid", false);
    let tile_addr = tile_addr.reg_next("stage_15_tile_addr");

    let r = r.reg_next("stage_15_r");
    let g = g.reg_next("stage_15_g");
    let b = b.reg_next("stage_15_b");
    let a = a.reg_next("stage_15_a");

    let z = z.reg_next("stage_15_z");

    let s = s.reg_next("stage_15_s");
    let t = t.reg_next("stage_15_t");

    let depth_test_result = depth_test_result.reg_next("stage_15_depth_test_result");

    let s_floor = s.bits(31, ST_FRACT_BITS);
    let t_floor = t.bits(31, ST_FRACT_BITS);
    let s_fract = m.low().concat(s.bits(ST_FRACT_BITS - 1, ST_FRACT_BITS - ST_FILTER_FRACT_BITS));
    let t_fract = m.low().concat(t.bits(ST_FRACT_BITS - 1, ST_FRACT_BITS - ST_FILTER_FRACT_BITS));
    let one_minus_s_fract = m.high().concat(m.lit(0u32, ST_FILTER_FRACT_BITS)) - s_fract;
    let one_minus_t_fract = m.high().concat(m.lit(0u32, ST_FILTER_FRACT_BITS)) - t_fract;

    //  Lock weights for nearest filtering
    let (s_fract, one_minus_s_fract, t_fract, one_minus_t_fract) = if_(!tex_filter_select, {
        let zero = m.low().concat(m.lit(0u32, ST_FILTER_FRACT_BITS));
        let one = m.high().concat(m.lit(0u32, ST_FILTER_FRACT_BITS));
        (zero, one, zero, one)
    }).else_({
        (s_fract, one_minus_s_fract, t_fract, one_minus_t_fract)
    });

    //  Swap weights depending on pixel offsets
    let (s_fract, one_minus_s_fract) = if_(!s_floor.bit(0), {
        (s_fract, one_minus_s_fract)
    }).else_({
        (one_minus_s_fract, s_fract)
    });
    let (t_fract, one_minus_t_fract) = if_(!t_floor.bit(0), {
        (t_fract, one_minus_t_fract)
    }).else_({
        (one_minus_t_fract, t_fract)
    });

    // Outputs
    m.output("out_valid", valid);
    m.output("out_tile_addr", tile_addr);

    m.output("out_r", r);
    m.output("out_g", g);
    m.output("out_b", b);
    m.output("out_a", a);

    m.output("out_z", z);

    m.output("out_depth_test_result", depth_test_result);

    m.output("out_s_fract", s_fract);
    m.output("out_one_minus_s_fract", one_minus_s_fract);
    m.output("out_t_fract", t_fract);
    m.output("out_one_minus_t_fract", one_minus_t_fract);

    let buffer0_s = (s_floor.bits(6, 0) + m.lit(1u32, 7)).bits(6, 1);
    let buffer0_t = (t_floor.bits(6, 0) + m.lit(1u32, 7)).bits(6, 1);
    let buffer1_s = s_floor.bits(6, 1);
    let buffer1_t = buffer0_t;
    let buffer2_s = buffer0_s;
    let buffer2_t = t_floor.bits(6, 1);
    let buffer3_s = buffer1_s;
    let buffer3_t = buffer2_t;
    let read_addr = |s: &'a Signal<'a>, t: &'a Signal<'a>, buffer_index: u32| {
        if_(tex_dim.eq(m.lit(REG_TEXTURE_SETTINGS_DIM_16, REG_TEXTURE_SETTINGS_DIM_BITS)), {
            tex_base
            .concat(m.lit(buffer_index, 2))
            .concat(t.bits(2, 0))
            .concat(s.bits(2, 0))
        }).else_if(tex_dim.eq(m.lit(REG_TEXTURE_SETTINGS_DIM_32, REG_TEXTURE_SETTINGS_DIM_BITS)), {
            tex_base
            .bits(6, 2)
            .concat(m.lit(buffer_index, 2))
            .concat(t.bits(3, 0))
            .concat(s.bits(3, 0))
        }).else_if(tex_dim.eq(m.lit(REG_TEXTURE_SETTINGS_DIM_64, REG_TEXTURE_SETTINGS_DIM_BITS)), {
            tex_base
            .bits(6, 4)
            .concat(m.lit(buffer_index, 2))
            .concat(t.bits(4, 0))
            .concat(s.bits(4, 0))
        }).else_({
            // REG_TEXTURE_SETTINGS_DIM_128
            tex_base
            .bit(6)
            .concat(m.lit(buffer_index, 2))
            .concat(t)
            .concat(s)
        })
    };
    m.output("out_tex_buffer0_read_addr", read_addr(buffer0_s, buffer0_t, 0));
    m.output("out_tex_buffer1_read_addr", read_addr(buffer1_s, buffer1_t, 1));
    m.output("out_tex_buffer2_read_addr", read_addr(buffer2_s, buffer2_t, 2));
    m.output("out_tex_buffer3_read_addr", read_addr(buffer3_s, buffer3_t, 3));

    m
}

pub fn generate_back_pipe<'a>(c: &'a Context<'a>) -> &Module<'a> {
    let m = c.module("BackPipe");

    // Aux inputs
    let depth_write_mask_enable = m.input("depth_write_mask_enable", 1);

    let blend_src_factor = m.input("blend_src_factor", REG_BLEND_SETTINGS_SRC_FACTOR_BITS);
    let blend_dst_factor = m.input("blend_dst_factor", REG_BLEND_SETTINGS_DST_FACTOR_BITS);

    // Inputs
    let valid = m.input("in_valid", 1);
    let tile_addr = m.input("in_tile_addr", TILE_PIXELS_BITS);

    let r = m.input("in_r", 9);
    let g = m.input("in_g", 9);
    let b = m.input("in_b", 9);
    let a = m.input("in_a", 9);

    let z = m.input("in_z", 16);

    let depth_test_result = m.input("in_depth_test_result", 1);

    let s_fract = m.input("in_s_fract", ST_FILTER_FRACT_BITS + 1);
    let one_minus_s_fract = m.input("in_one_minus_s_fract", ST_FILTER_FRACT_BITS + 1);
    let t_fract = m.input("in_t_fract", ST_FILTER_FRACT_BITS + 1);
    let one_minus_t_fract = m.input("in_one_minus_t_fract", ST_FILTER_FRACT_BITS + 1);
    struct Texel<'a> {
        r: &'a Signal<'a>,
        g: &'a Signal<'a>,
        b: &'a Signal<'a>,
        a: &'a Signal<'a>,
    }

    impl<'a> Texel<'a> {
        fn new<S: Into<&'a Signal<'a>>>(texel: S) -> Texel<'a> {
            let texel = texel.into();
            Texel {
                r: texel.bits(23, 16),
                g: texel.bits(15, 8),
                b: texel.bits(7, 0),
                a: texel.bits(31, 24),
            }
        }

        fn argb(&self) -> &'a Signal<'a> {
            self.a.concat(self.r).concat(self.g).concat(self.b)
        }
    }

    let texel0 = Texel::new(m.input("in_tex_buffer0_read_value", 32));
    let texel1 = Texel::new(m.input("in_tex_buffer1_read_value", 32));
    let texel2 = Texel::new(m.input("in_tex_buffer2_read_value", 32));
    let texel3 = Texel::new(m.input("in_tex_buffer3_read_value", 32));

    // Stage 1
    fn blend_component<'a, S1: Into<&'a Signal<'a>>, S2: Into<&'a Signal<'a>>>(
        a: &'a Signal<'a>,
        b: &'a Signal<'a>,
        a_fract: S1,
        b_fract: S2,
    ) -> &'a Signal<'a> {
        (a * a_fract + b * b_fract).bits(8 + ST_FILTER_FRACT_BITS - 1, ST_FILTER_FRACT_BITS)
    }

    fn blend_texels<'a, S1: Into<&'a Signal<'a>>, S2: Into<&'a Signal<'a>>>(
        a: &Texel<'a>,
        b: &Texel<'a>,
        a_fract: S1,
        b_fract: S2,
    ) -> Texel<'a> {
        Texel {
            r: blend_component(a.r, b.r, a_fract, b_fract),
            g: blend_component(a.g, b.g, a_fract, b_fract),
            b: blend_component(a.b, b.b, a_fract, b_fract),
            a: blend_component(a.a, b.a, a_fract, b_fract),
        }
    }

    let lower = blend_texels(&texel0, &texel1, one_minus_s_fract, s_fract).argb();
    let upper = blend_texels(&texel2, &texel3, one_minus_s_fract, s_fract).argb();

    // Stage 2
    let valid = valid.reg_next_with_default("stage_2_valid", false);
    let tile_addr = tile_addr.reg_next("stage_2_tile_addr");

    let r = r.reg_next("stage_2_r");
    let g = g.reg_next("stage_2_g");
    let b = b.reg_next("stage_2_b");
    let a = a.reg_next("stage_2_a");

    let z = z.reg_next("stage_2_z");

    let depth_test_result = depth_test_result.reg_next("stage_2_depth_test_result");

    let t_fract = t_fract.reg_next("stage_2_t_fract");
    let one_minus_t_fract = one_minus_t_fract.reg_next("stage_2_one_minus_t_fract");

    let lower = Texel::new(lower.reg_next("stage_2_lower"));
    let upper = Texel::new(upper.reg_next("stage_2_upper"));

    let texel = blend_texels(&lower, &upper, one_minus_t_fract, t_fract).argb();

    // Stage 3
    let valid = valid.reg_next_with_default("stage_3_valid", false);
    let tile_addr = tile_addr.reg_next("stage_3_tile_addr");

    let r = r.reg_next("stage_3_r");
    let g = g.reg_next("stage_3_g");
    let b = b.reg_next("stage_3_b");
    let a = a.reg_next("stage_3_a");

    let z = z.reg_next("stage_3_z");

    let depth_test_result = depth_test_result.reg_next("stage_3_depth_test_result");

    let texel = Texel::new(texel.reg_next("stage_3_texel"));

    let scale_comp = |color_comp: &'a Signal<'a>, texel_comp: &'a Signal<'a>| -> &'a Signal<'a> {
        (color_comp * texel_comp).bits(16, 8)
    };

    let r = scale_comp(r, texel.r);
    let g = scale_comp(g, texel.g);
    let b = scale_comp(b, texel.b);
    let a = scale_comp(a, texel.a);

    //  Issue color buffer read for prev_color
    m.output("color_buffer_read_port_addr", tile_addr.bits(TILE_PIXELS_BITS - 1, 2));
    m.output("color_buffer_read_port_enable", valid);

    // Stage 4
    let valid = valid.reg_next_with_default("stage_4_valid", false);
    let tile_addr = tile_addr.reg_next("stage_4_tile_addr");

    let r = r.reg_next("stage_4_r");
    let g = g.reg_next("stage_4_g");
    let b = b.reg_next("stage_4_b");
    let a = a.reg_next("stage_4_a");

    let z = z.reg_next("stage_4_z");

    let depth_test_result = depth_test_result.reg_next("stage_4_depth_test_result");

    let zero = m.lit(0u32, 9);
    let one = m.high().concat(m.lit(0u32, 8));

    let blend_src_factor = if_(blend_src_factor.eq(m.lit(REG_BLEND_SETTINGS_SRC_FACTOR_ZERO, REG_BLEND_SETTINGS_SRC_FACTOR_BITS)), {
        zero
    }).else_if(blend_src_factor.eq(m.lit(REG_BLEND_SETTINGS_SRC_FACTOR_ONE, REG_BLEND_SETTINGS_SRC_FACTOR_BITS)), {
        one
    }).else_if(blend_src_factor.eq(m.lit(REG_BLEND_SETTINGS_SRC_FACTOR_SRC_ALPHA, REG_BLEND_SETTINGS_SRC_FACTOR_BITS)), {
        a
    }).else_({
        one - a
    });

    let blend_dst_factor = if_(blend_dst_factor.eq(m.lit(REG_BLEND_SETTINGS_DST_FACTOR_ZERO, REG_BLEND_SETTINGS_DST_FACTOR_BITS)), {
        zero
    }).else_if(blend_dst_factor.eq(m.lit(REG_BLEND_SETTINGS_DST_FACTOR_ONE, REG_BLEND_SETTINGS_DST_FACTOR_BITS)), {
        one
    }).else_if(blend_dst_factor.eq(m.lit(REG_BLEND_SETTINGS_DST_FACTOR_SRC_ALPHA, REG_BLEND_SETTINGS_DST_FACTOR_BITS)), {
        a
    }).else_({
        one - a
    });

    //  Returned from issue in previous stage
    let prev_color = m.input("color_buffer_read_port_value", 128);
    let prev_color = if_(tile_addr.bits(1, 0).eq(m.lit(0u32, 2)), {
        prev_color.bits(31, 0)
    }).else_if(tile_addr.bits(1, 0).eq(m.lit(1u32, 2)), {
        prev_color.bits(63, 32)
    }).else_if(tile_addr.bits(1, 0).eq(m.lit(2u32, 2)), {
        prev_color.bits(95, 64)
    }).else_({
        prev_color.bits(127, 96)
    });

    // Stage 5
    let valid = valid.reg_next_with_default("stage_5_valid", false);
    let tile_addr = tile_addr.reg_next("stage_5_tile_addr");

    let r = r.reg_next("stage_5_r");
    let g = g.reg_next("stage_5_g");
    let b = b.reg_next("stage_5_b");
    let a = a.reg_next("stage_5_a");

    let z = z.reg_next("stage_5_z");

    let depth_test_result = depth_test_result.reg_next("stage_5_depth_test_result");

    let blend_src_factor = blend_src_factor.reg_next("stage_5_blend_src_factor");
    let blend_dst_factor = blend_dst_factor.reg_next("stage_5_blend_dst_factor");

    let prev_color = prev_color.reg_next("stage_5_prev_color");

    let r = (r * blend_src_factor).bits(17, 8);
    let g = (g * blend_src_factor).bits(17, 8);
    let b = (b * blend_src_factor).bits(17, 8);

    let prev_r = m.lit(0u32, 2).concat((prev_color.bits(23, 16) * blend_dst_factor).bits(16, 9));
    let prev_g = m.lit(0u32, 2).concat((prev_color.bits(15, 8) * blend_dst_factor).bits(16, 9));
    let prev_b = m.lit(0u32, 2).concat((prev_color.bits(7, 0) * blend_dst_factor).bits(16, 9));

    let clamp_comp = |comp: &'a Signal<'a>| -> &'a Signal<'a> {
        if_(comp.bits(9, 8).eq(m.lit(0u32, 2)), {
            comp.bits(7, 0)
        }).else_({
            m.lit(255u32, 8)
        })
    };

    let r = clamp_comp(r + prev_r);
    let g = clamp_comp(g + prev_g);
    let b = clamp_comp(b + prev_b);
    let a = clamp_comp(m.low().concat(a));

    let color = a.concat(r).concat(g).concat(b);

    // Stage 6
    let valid = valid.reg_next_with_default("stage_6_valid", false);
    let tile_addr = tile_addr.reg_next("stage_6_tile_addr");

    let z = z.reg_next("stage_6_z");

    let depth_test_result = depth_test_result.reg_next("stage_6_depth_test_result");

    let color = color.reg_next("stage_6_color");

    m.output("color_buffer_write_port_addr", tile_addr.bits(TILE_PIXELS_BITS - 1, 2));
    m.output("color_buffer_write_port_value", color.repeat(4));
    m.output("color_buffer_write_port_enable", valid & depth_test_result);
    m.output("color_buffer_write_port_word_enable", (0u32..4).fold(None, |acc, x| {
        let word_enable_bit = tile_addr.bits(1, 0).eq(m.lit(x, 2));
        Some(if let Some(acc) = acc {
            word_enable_bit.concat(acc)
        } else {
            word_enable_bit
        })
    }).unwrap());

    m.output("depth_buffer_write_port_addr", tile_addr.bits(TILE_PIXELS_BITS - 1, 3));
    m.output("depth_buffer_write_port_value", z.repeat(8));
    m.output("depth_buffer_write_port_enable", valid & depth_test_result & depth_write_mask_enable);
    m.output("depth_buffer_write_port_word_enable", (0u32..8).fold(None, |acc, x| {
        let word_enable_bit = tile_addr.bits(2, 0).eq(m.lit(x, 3));
        Some(if let Some(acc) = acc {
            word_enable_bit.concat(acc)
        } else {
            word_enable_bit
        })
    }).unwrap());

    // Outputs
    m.output("out_valid", valid);

    m
}
