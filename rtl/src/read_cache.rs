use crate::buster::*;

use kaze::*;

// We're going to drive some mems' read ports' enable signals with logic that includes those read ports'
//  registered output values, which is not possible with the current kaze Mem API. To get around this, we'll
//  create a `wire` construct (similar to a Verilog `wire`) which will allow us to use these output values
//  symbolically before binding their actual values.
// This still results in a valid signal graph because memory elements behave as registers and thus don't
//  form combinational loops.
pub struct Wire<'a> {
    pub m: &'a Module<'a>,
    pub i: &'a Input<'a>,
    pub o: &'a Output<'a>,
}

impl<'a> Wire<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(instance_name: S, bit_width: u32, p: &'a P) -> Wire<'a> {
        let m = p.module(instance_name, "Wire");

        let i = m.input("i", bit_width);
        let o = m.output("o", i);

        Wire {
            m,
            i,
            o,
        }
    }
}

pub struct ReadCache<'a> {
    pub m: &'a Module<'a>,
    pub invalidate: &'a Input<'a>,
    pub primary_port: PrimaryPort<'a>,
    pub replica_port: ReplicaPort<'a>,
}

impl<'a> ReadCache<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(
        instance_name: S,
        data_bit_width: u32,
        addr_bit_width: u32,
        cache_addr_bit_width: u32,
        p: &'a P,
    ) -> ReadCache<'a> {
        // TODO: Ensure cache_addr_bit_width is less than addr_bit_width

        let m = p.module(instance_name, "ReadCache");

        let tag_bit_width = addr_bit_width - cache_addr_bit_width;

        let valid_mem = m.mem("valid", cache_addr_bit_width, 1);
        let tag_mem = m.mem("tag", cache_addr_bit_width, tag_bit_width);
        let data_mem = m.mem("data", cache_addr_bit_width, data_bit_width);

        let valid_mem_read_port_value_wire = Wire::new("valid_mem_read_port_value_wire", 1, m);
        let tag_mem_read_port_value_wire = Wire::new("tag_mem_read_port_value_wire", tag_bit_width, m);

        let state_bit_width = 2;
        let state_invalidate = 0u32;
        let state_active = 1u32;
        let state_miss_return = 2u32;
        let state = m.reg("state", state_bit_width);
        state.default_value(state_invalidate);

        let invalidate = m.input("invalidate", 1);
        let invalidate_queued = m.reg("invalidate_queued", 1);
        invalidate_queued.default_value(false);
        let will_invalidate = invalidate | invalidate_queued;

        let invalidate_addr = m.reg("invalidate_addr", cache_addr_bit_width);
        invalidate_addr.default_value(0u32);

        let primary_bus_enable = m.input("primary_bus_enable", 1);
        let primary_bus_addr = m.input("primary_bus_addr", addr_bit_width);
        let cache_addr = primary_bus_addr.bits(cache_addr_bit_width - 1, 0);

        let issue_buffer_occupied = m.reg("issue_buffer_occupied", 1);
        issue_buffer_occupied.default_value(false);

        let issue_buffer_addr = m.reg("issue_buffer_addr", addr_bit_width);
        let issue_buffer_tag = issue_buffer_addr.bits(addr_bit_width - 1, cache_addr_bit_width);
        let issue_buffer_cache_addr = issue_buffer_addr.bits(cache_addr_bit_width - 1, 0);

        let replica_bus_ready = m.input("replica_bus_ready", 1);
        let replica_bus_read_data = m.input("replica_bus_read_data", data_bit_width);
        let replica_bus_read_data_valid = m.input("replica_bus_read_data_valid", 1);

        // A mem read that occurs simultaneously with a write to the same location will return the *previous* value
        //  at that location, *not* the new one from the write.
        // This is problematic for the special case where we're currently receiving data from the replica (and
        //  returning it to the primary) *and* the primary is issuing a read from the same location.
        // Since we're writing to the internal mems at the same location that the request will read from this cycle,
        //  the read will return stale data!
        // To work around this, we introduce a bypass mechanism which detects this specific case (exactly as described
        //  above) and overrides *both* hit detection and returned data on the following cycle. This is sufficient for
        //  all cases since the cache memory will be up-to-date on the cycle after the bypass cycle again.
        // Note that if we ignored this case, the cache would still return correct data, but only after erroneously
        //  detecting a miss and issuing a redundant read to the replica and waiting for it to return again - so at
        //  a system level, this fixes a performance bug, not a logical one... though, for a cache, this is probably
        //  not a useful distinction!
        let internal_mem_bypass =
            (replica_bus_read_data_valid & primary_bus_enable & primary_bus_addr.eq(issue_buffer_addr))
            .reg_next_with_default(
                "internal_mem_bypass",
                false);

        let issue_buffer_valid = (valid_mem_read_port_value_wire.o & tag_mem_read_port_value_wire.o.eq(issue_buffer_tag)) | internal_mem_bypass;

        let hit = issue_buffer_occupied & issue_buffer_valid;
        let miss = issue_buffer_occupied & !issue_buffer_valid;

        // TODO: Simplify?
        let can_accept_issue =
            (state.eq(m.lit(state_active, state_bit_width)) & (!issue_buffer_occupied | hit)) |
            (state.eq(m.lit(state_miss_return, state_bit_width)) & replica_bus_read_data_valid);
        let can_accept_issue = can_accept_issue & !will_invalidate;

        let primary_bus_ready = m.output("primary_bus_ready", can_accept_issue);

        let accept_issue = can_accept_issue & primary_bus_enable;

        valid_mem_read_port_value_wire.i.drive(valid_mem.read_port(cache_addr, accept_issue));
        tag_mem_read_port_value_wire.i.drive(tag_mem.read_port(cache_addr, accept_issue));

        issue_buffer_occupied.drive_next(if_(replica_bus_read_data_valid | !miss, {
            accept_issue
        }).else_({
            issue_buffer_occupied
        }));

        issue_buffer_addr.drive_next(if_(accept_issue, {
            primary_bus_addr
        }).else_({
            issue_buffer_addr
        }));

        let start_invalidate = will_invalidate & !issue_buffer_occupied;

        invalidate_queued.drive_next(if_(start_invalidate | state.eq(m.lit(state_invalidate, state_bit_width)), {
            m.low()
        }).else_if(invalidate, {
            m.high()
        }).else_({
            invalidate_queued
        }));

        invalidate_addr.drive_next(if_(start_invalidate, {
            m.lit(0u32, cache_addr_bit_width)
        }).else_({
            invalidate_addr + m.lit(1u32, cache_addr_bit_width)
        }));

        let replica_bus_enable = m.output("replica_bus_enable", state.eq(m.lit(state_active, state_bit_width)) & miss);
        let replica_bus_addr = m.output("replica_bus_addr", issue_buffer_addr);
        let primary_bus_read_data = m.output("primary_bus_read_data", if_(replica_bus_read_data_valid, {
            replica_bus_read_data.into()
        }).else_if(internal_mem_bypass, {
            replica_bus_read_data.reg_next("internal_mem_bypass_data")
        }).else_({
            data_mem.read_port(cache_addr, accept_issue)
        }));
        let primary_bus_read_data_valid = m.output("primary_bus_read_data_valid", replica_bus_read_data_valid | hit);

        state.drive_next(if_(start_invalidate, {
            m.lit(state_invalidate, state_bit_width)
        }).else_({
            if_(state.eq(m.lit(state_invalidate, state_bit_width)), {
                if_(invalidate_addr.eq(m.lit((1u32 << cache_addr_bit_width) - 1, cache_addr_bit_width)), {
                    m.lit(state_active, state_bit_width)
                }).else_({
                    state
                })
            }).else_if(state.eq(m.lit(state_active, state_bit_width)), {
                if_(miss & replica_bus_ready, {
                    m.lit(state_miss_return, state_bit_width)
                }).else_({
                    state
                })
            }).else_({
                // state_miss_return
                if_(replica_bus_read_data_valid, {
                    m.lit(state_active, state_bit_width)
                }).else_({
                    state
                })
            })
        }));

        valid_mem.write_port(
            if_(replica_bus_read_data_valid, {
                issue_buffer_cache_addr
            }).else_({
                invalidate_addr
            }),
            replica_bus_read_data_valid,
            replica_bus_read_data_valid | state.eq(m.lit(state_invalidate, state_bit_width)));
        tag_mem.write_port(
            issue_buffer_cache_addr,
            issue_buffer_tag,
            replica_bus_read_data_valid);
        data_mem.write_port(
            issue_buffer_cache_addr,
            replica_bus_read_data,
            replica_bus_read_data_valid);

        ReadCache {
            m,
            invalidate,
            primary_port: PrimaryPort {
                bus_enable: primary_bus_enable,
                bus_addr: primary_bus_addr,
                bus_write: m.input("primary_bus_write", 1),
                bus_write_data: m.input("primary_bus_write_data", data_bit_width),
                bus_write_byte_enable: m.input("primary_bus_write_byte_enable", data_bit_width / 8),
                bus_ready: primary_bus_ready,
                bus_read_data: primary_bus_read_data,
                bus_read_data_valid: primary_bus_read_data_valid,
            },
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
        }
    }
}
