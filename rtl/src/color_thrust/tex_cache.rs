use crate::buster::*;
use crate::read_cache::*;
use super::*;

use kaze::*;

use std::collections::HashMap;

pub struct TexCache<'a> {
    pub m: &'a Module<'a>,

    invalidate: &'a Input<'a>,
    in_valid: &'a Input<'a>,
    in_ready: &'a Output<'a>,
    out_valid: &'a Output<'a>,

    in_tex_buffer_read_addrs: Vec<&'a Input<'a>>,
    out_tex_buffer_read_values: Vec<&'a Output<'a>>,

    replica_port: ReplicaPort<'a>,

    forward_inputs: HashMap<String, &'a Input<'a>>,
    forward_outputs: HashMap<String, &'a Output<'a>>,
}

impl<'a> TexCache<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> TexCache<'a> {
        let m = p.module(instance_name, "TexCache");

        let invalidate = m.input("invalidate", 1);

        let in_valid = m.input("in_valid", 1);

        let issue_buffer_occupied = m.reg("issue_buffer_occupied", 1);
        issue_buffer_occupied.default_value(false);

        let block_cache_crossbar = Crossbar::new("block_cache_crossbar", 4, 1, TEX_WORD_ADDR_BITS, 0, 128, 5, m);
        let replica_port = block_cache_crossbar.replica_ports[0].forward("replica", m);

        let mut in_tex_buffer_read_addrs = Vec::new();
        let mut out_tex_buffer_read_values = Vec::new();
        let mut acc = None;
        let block_caches = (0..4).map(|i| {
            let block_cache = BlockCache::new(format!("block_cache{}", i), m);
            block_cache.invalidate.drive(invalidate);
            let addr = m.input(format!("in_tex_buffer{}_read_addr", i), TEX_PIXEL_ADDR_BITS);
            block_cache.in_addr.drive(addr);
            let return_data = block_cache.return_data;
            let value = m.output(format!("out_tex_buffer{}_read_value", i), return_data);

            in_tex_buffer_read_addrs.push(addr);
            out_tex_buffer_read_values.push(value);

            block_cache_crosspar.primary_ports[i].connect(&block_cache.replica_port);
            let in_ready = block_cache.in_ready.into();
            let return_data_valid = block_cache.return_data_valid.into();
            acc = Some(match acc {
                Some((acc_in_ready, acc_return_data_valid)) => (acc_in_ready & in_ready, acc_return_data_valid & return_data_valid),
                _ => (in_ready, return_data_valid)
            });

            block_cache
        }).collect::<Vec<_>>();
        let (caches_in_ready, caches_return_data_valid) = acc.unwrap();

        let out_valid = issue_buffer_occupied & caches_return_data_valid;

        //  Note that we exploit implementation details of `ReadCache` - namely that we
        //   know that its `primary_bus_ready` output is independent of its
        //   `replica_bus_ready` input, so regardless of arbitration or whatever else we
        //   connect between the caches on the replica side (which may introduce some
        //   interdepencies), we know that they can be in a state where all of them can
        //   accept reads simultaneously. This simplifies issue logic in this cache.
        let can_accept_issue = caches_in_ready & (!issue_buffer_occupied | caches_return_data_valid);
        let in_ready = m.output("in_ready", can_accept_issue);

        let accept_issue = can_accept_issue & in_valid;

        issue_buffer_occupied.drive_next(if_(accept_issue, {
            m.high()
        }).else_if(out_valid, {
            m.low()
        }).else_({
            issue_buffer_occupied
        }));

        for block_cache in block_caches {
            block_cache.issue.drive(accept_issue);
        }

        let mut forward_inputs = HashMap::new();
        let mut forward_outputs = HashMap::new();
        for (name, bit_width) in [
            ("tile_addr", TILE_PIXELS_BITS),

            ("r", 9),
            ("g", 9),
            ("b", 9),
            ("a", 9),

            ("z", 16),

            ("depth_test_result", 1),

            ("s_fract", ST_FILTER_FRACT_BITS + 1),
            ("one_minus_s_fract", ST_FILTER_FRACT_BITS + 1),
            ("t_fract", ST_FILTER_FRACT_BITS + 1),
            ("one_minus_t_fract", ST_FILTER_FRACT_BITS + 1),
        ].iter() {
            let input = m.input(format!("in_{}", name), *bit_width);
            let reg = m.reg(format!("{}_forward", name), *bit_width);
            reg.drive_next(if_(accept_issue, {
                input
            }).else_({
                reg
            }));
            let output = m.output(format!("out_{}", name), reg);
            forward_inputs.insert(*name.into(), input);
            forward_outputs.insert(*name.into(), output);
        }

        TexCache {
            m,

            invalidate,
            in_valid,
            in_ready,
            out_valid: m.output("out_valid", out_valid),

            in_tex_buffer_read_addrs,
            out_tex_buffer_read_values,

            replica_port,

            forward_inputs,
            forward_outputs,
        }
    }
}

pub struct BlockCache<'a> {
    pub m: &'a Module<'a>,

    pub invalidate: &'a Input<'a>,

    pub replica_port: ReplicaPort<'a>,
    pub issue: &'a Input<'a>,
    pub in_ready: &'a Output<'a>,
    pub in_addr: &'a Input<'a>,
    pub return_data: &'a Output<'a>,
    pub return_data_valid: &'a Output<'a>,
}

impl<'a> BlockCache<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, p: &'a P) -> BlockCache<'a> {
        let m = p.module(instance_name, "BlockCache");

        let invalidate = m.input("invalidate", 1);

        // TODO: Properly expose/check these parameters!
        let read_cache = ReadCache::new("read_cache", 128, TEX_WORD_ADDR_BITS, TEX_WORD_ADDR_BITS - 3, m);

        read_cache.invalidate.drive(invalidate);

        let replica_bus_ready = m.input("replica_bus_ready", 1);
        read_cache.replica_port.bus_ready.drive(replica_bus_ready);
        let replica_bus_enable = m.output("replica_bus_enable", read_cache.replica_port.bus_enable);
        let replica_bus_addr = m.output("replica_bus_addr", read_cache.replica_port.bus_addr);
        let replica_bus_read_data = m.input("replica_bus_read_data", 128);
        read_cache.replica_port.bus_read_data.drive(replica_bus_read_data);
        let replica_bus_read_data_valid = m.input("replica_bus_read_data_valid", 1);
        read_cache.replica_port.bus_read_data_valid.drive(replica_bus_read_data_valid);

        let issue = m.input("issue", 1);
        let in_ready = m.output("in_ready", read_cache.primary_port.bus_ready);
        read_cache.primary_port.bus_enable.drive(issue);
        let in_addr = m.input("in_addr", TEX_PIXEL_ADDR_BITS);
        read_cache.primary_port.bus_addr.drive(in_addr.bits(TEX_PIXEL_ADDR_BITS - 1, 2));

        let pixel_sel = m.reg("pixel_sel", 2);
        pixel_sel.drive_next(if_(issue, {
            in_addr.bits(1, 0)
        }).else_({
            pixel_sel
        }));

        let read_data = read_cache.primary_port.bus_read_data;
        let read_pixel = if_(pixel_sel.eq(m.lit(0u32, 2)), {
            read_data.bits(31, 0)
        }).else_if(pixel_sel.eq(m.lit(1u32, 2)), {
            read_data.bits(63, 32)
        }).else_if(pixel_sel.eq(m.lit(2u32, 2)), {
            read_data.bits(95, 64)
        }).else_({
            read_data.bits(127, 96)
        });

        let read_data_valid = read_cache.primary_port.bus_read_data_valid;

        let return_buffer_occupied = m.reg("return_buffer_occupied", 1);
        return_buffer_occupied.default_value(false);
        return_buffer_occupied.drive_next(if_(issue, {
            m.low()
        }).else_if(read_data_valid, {
            m.high()
        }).else_({
            return_buffer_occupied
        }));

        let return_buffer_pixel = m.reg("return_buffer_pixel", 32);
        return_buffer_pixel.drive_next(if_(read_data_valid, {
            read_pixel
        }).else_({
            return_buffer_pixel
        }));

        let return_data = m.output("return_data", if_(read_data_valid, {
            read_pixel
        }).else_({
            return_buffer_pixel
        }));
        let return_data_valid = m.output("return_data_valid", read_data_valid | return_buffer_occupied);

        BlockCache {
            m,

            invalidate,

            replica_port: ReplicaPort {
                bus_enable: replica_bus_enable,
                bus_addr: replica_bus_addr,
                bus_write: m.output("replica_bus_write", m.low()),
                bus_write_data: m.output("replica_bus_write_data", m.lit(0u32, data_bit_width)),
                bus_write_byte_enable: m.output("replica_bus_write_byte_enable", m.lit(0u32, data_bit_width / 8)),
                bus_ready: replica_bus_ready,
                bus_read_data: replica_bus_read_data,
                bus_read_data_valid: replica_bus_read_data_valid,
            },
            issue,
            in_ready,
            in_addr,
            return_data,
            return_data_valid,
        }
    }
}
