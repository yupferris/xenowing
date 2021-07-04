use crate::fifo::*;
use crate::peek_buffer::*;

use kaze::*;

pub struct PrimaryPort<'a> {
    pub bus_enable: &'a Input<'a>,
    pub bus_addr: &'a Input<'a>,
    pub bus_write: &'a Input<'a>,
    pub bus_write_data: &'a Input<'a>,
    pub bus_write_byte_enable: &'a Input<'a>,
    pub bus_ready: &'a Output<'a>,
    pub bus_read_data: &'a Output<'a>,
    pub bus_read_data_valid: &'a Output<'a>,
}

impl<'a> PrimaryPort<'a> {
    pub fn connect(&self, replica_port: &ReplicaPort<'a>) {
        // TODO: Consider higher-level sanity checks here before connecting to improve error reporting
        self.bus_enable.drive(replica_port.bus_enable);
        self.bus_addr.drive(replica_port.bus_addr);
        self.bus_write.drive(replica_port.bus_write);
        self.bus_write_data.drive(replica_port.bus_write_data);
        self.bus_write_byte_enable.drive(replica_port.bus_write_byte_enable);
        replica_port.bus_ready.drive(self.bus_ready);
        replica_port.bus_read_data.drive(self.bus_read_data);
        replica_port.bus_read_data_valid.drive(self.bus_read_data_valid);
    }
}

pub struct ReplicaPort<'a> {
    pub bus_enable: &'a Output<'a>,
    pub bus_addr: &'a Output<'a>,
    pub bus_write: &'a Output<'a>,
    pub bus_write_data: &'a Output<'a>,
    pub bus_write_byte_enable: &'a Output<'a>,
    pub bus_ready: &'a Input<'a>,
    pub bus_read_data: &'a Input<'a>,
    pub bus_read_data_valid: &'a Input<'a>,
}

impl<'a> ReplicaPort<'a> {
    pub fn forward<S: Into<String>>(&self, name_prefix: S, m: &'a Module) -> ReplicaPort<'a> {
        let name_prefix = name_prefix.into();
        let bus_ready = m.input(format!("{}_bus_ready", name_prefix), self.bus_ready.bit_width());
        self.bus_ready.drive(bus_ready);
        let bus_read_data = m.input(format!("{}_bus_read_data", name_prefix), self.bus_read_data.bit_width());
        self.bus_read_data.drive(bus_read_data);
        let bus_read_data_valid = m.input(format!("{}_bus_read_data_valid", name_prefix), self.bus_read_data_valid.bit_width());
        self.bus_read_data_valid.drive(bus_read_data_valid);
        ReplicaPort {
            bus_enable: m.output(format!("{}_bus_enable", name_prefix), self.bus_enable),
            bus_addr: m.output(format!("{}_bus_addr", name_prefix), self.bus_addr),
            bus_write: m.output(format!("{}_bus_write", name_prefix), self.bus_write),
            bus_write_data: m.output(format!("{}_bus_write_data", name_prefix), self.bus_write_data),
            bus_write_byte_enable: m.output(format!("{}_bus_write_byte_enable", name_prefix), self.bus_write_byte_enable),
            bus_ready,
            bus_read_data,
            bus_read_data_valid,
        }
    }
}

pub struct Crossbar<'a> {
    pub m: &'a Module<'a>,
    pub primary_ports: Vec<PrimaryPort<'a>>,
    pub replica_ports: Vec<ReplicaPort<'a>>,
}

impl<'a> Crossbar<'a> {
    pub fn new<S: Into<String>, P: ModuleParent<'a>>(
        instance_name: S,
        num_primaries: u32,
        num_replicas: u32,
        addr_bit_width: u32,
        replica_select_bit_width: u32,
        data_bit_width: u32,
        fifo_depth_bits: u32,
        p: &'a P,
    ) -> Crossbar<'a> {
        if num_primaries == 0 {
            panic!("Cannot generate a buster crossbar module with zero primaries.");
        }
        if num_replicas == 0 {
            panic!("Cannot generate a buster crossbar module with zero replicas.");
        }

        // TODO: num_primaries, num_replicas, replica_select_bit_width bounds checks
        let primary_select_bit_width = 31 - num_primaries.leading_zeros();
        let replica_addr_bit_width = addr_bit_width - replica_select_bit_width; // TODO: Bounds checks

        let data_byte_width = data_bit_width / 8;

        let m = p.module(instance_name, "Crossbar");

        let issue_arbiter = IssueArbiter::new(
            "issue_arbiter",
            num_primaries,
            addr_bit_width,
            data_bit_width,
            data_byte_width,
            primary_select_bit_width,
            m,
        );
        let primary_issues = (0..num_primaries).map(|i| {
            // TODO: This turned out ugly, could it be improved?
            let issue_arbiter_primary_issue = &issue_arbiter.primary_issues[i as usize];
            let name = format!("primary{}", i);
            let ret = PrimaryIssue {
                name: name.clone(),
                bus_enable: m.input(format!("{}_bus_enable", name), 1),
                bus_addr: m.input(format!("{}_bus_addr", name), addr_bit_width),
                bus_write: m.input(format!("{}_bus_write", name), 1),
                bus_write_data: m.input(format!("{}_bus_write_data", name), data_bit_width),
                bus_write_byte_enable: m.input(format!("{}_bus_write_byte_enable", name), data_byte_width),
                bus_ready: m.output(format!("{}_bus_ready", name), issue_arbiter_primary_issue.bus_ready),
            };
            issue_arbiter_primary_issue.bus_enable.drive(ret.bus_enable);
            issue_arbiter_primary_issue.bus_addr.drive(ret.bus_addr);
            issue_arbiter_primary_issue.bus_write.drive(ret.bus_write);
            issue_arbiter_primary_issue.bus_write_data.drive(ret.bus_write_data);
            issue_arbiter_primary_issue.bus_write_byte_enable.drive(ret.bus_write_byte_enable);
            ret
        }).collect::<Vec<_>>();

        let issue = Issue::new(
            "issue",
            num_primaries,
            num_replicas,
            addr_bit_width,
            replica_select_bit_width,
            data_bit_width,
            data_byte_width,
            primary_select_bit_width,
            replica_addr_bit_width,
            m,
        );
        issue.issue_arb_bus_enable.drive(issue_arbiter.issue_bus_enable);
        issue.issue_arb_bus_addr.drive(issue_arbiter.issue_bus_addr);
        issue.issue_arb_bus_write.drive(issue_arbiter.issue_bus_write);
        issue.issue_arb_bus_write_data.drive(issue_arbiter.issue_bus_write_data);
        issue.issue_arb_bus_write_byte_enable.drive(issue_arbiter.issue_bus_write_byte_enable);
        issue_arbiter.issue_bus_ready.drive(issue.issue_arb_bus_ready);
        let replica_issues = (0..num_replicas).map(|i| {
            let issue_replica_issue = &issue.replica_issues[i as usize];
            let name = format!("replica{}", i);
            let ret = ReplicaIssue {
                name: name.clone(),
                bus_enable: m.output(format!("{}_bus_enable", name), issue_replica_issue.bus_enable),
                bus_addr: m.output(format!("{}_bus_addr", name), issue_replica_issue.bus_addr),
                bus_write: m.output(format!("{}_bus_write", name), issue_replica_issue.bus_write),
                bus_write_data: m.output(format!("{}_bus_write_data", name), issue_replica_issue.bus_write_data),
                bus_write_byte_enable: m.output(format!("{}_bus_write_byte_enable", name), issue_replica_issue.bus_write_byte_enable),
                bus_ready: m.input(format!("{}_bus_ready", name), 1),
            };
            issue_replica_issue.bus_ready.drive(ret.bus_ready);
            ret
        }).collect::<Vec<_>>();

        let return_arbiter = ReturnArbiter::new(
            "return_arbiter",
            num_primaries,
            num_replicas,
            replica_select_bit_width,
            data_bit_width,
            primary_select_bit_width,
            m,
        );
        let primary_ports = (0..num_primaries).map(|i| {
            let primary_issue = &primary_issues[i as usize];
            let bus_read_data = return_arbiter.primary_bus_read_data_outputs[i as usize];
            let bus_read_data_valid = return_arbiter.primary_bus_read_data_valid_outputs[i as usize];
            PrimaryPort {
                bus_enable: primary_issue.bus_enable,
                bus_addr: primary_issue.bus_addr,
                bus_write: primary_issue.bus_write,
                bus_write_data: primary_issue.bus_write_data,
                bus_write_byte_enable: primary_issue.bus_write_byte_enable,
                bus_ready: primary_issue.bus_ready,
                bus_read_data: m.output(format!("primary{}_bus_read_data", i), bus_read_data),
                bus_read_data_valid: m.output(format!("primary{}_bus_read_data_valid", i), bus_read_data_valid),
            }
        }).collect::<Vec<_>>();

        if num_primaries > 1 {
            let primary_fifo = Fifo::new("primary_fifo", fifo_depth_bits, primary_select_bit_width, m);
            issue.issue_arb_bus_primary.unwrap().drive(issue_arbiter.issue_bus_primary.unwrap());
            issue.primary_fifo_full.unwrap().drive(primary_fifo.full);
            primary_fifo.write_enable.drive(issue.primary_fifo_write_enable.unwrap());
            primary_fifo.write_data.drive(issue.primary_fifo_write_data.unwrap());
            return_arbiter.primary_fifo_empty.unwrap().drive(primary_fifo.empty);
            return_arbiter.primary_fifo_read_data.unwrap().drive(primary_fifo.read_data);
            primary_fifo.read_enable.drive(return_arbiter.primary_fifo_read_enable);
        }

        if num_replicas > 1 {
            let replica_fifo = Fifo::new("replica_fifo", fifo_depth_bits, replica_select_bit_width, m);
            issue.replica_fifo_full.unwrap().drive(replica_fifo.full);
            replica_fifo.write_enable.drive(issue.replica_fifo_write_enable.unwrap());
            replica_fifo.write_data.drive(issue.replica_fifo_write_data.unwrap());

            let replica_buffer = PeekBuffer::new("replica_buffer", replica_select_bit_width, m);
            replica_buffer.ingress_data.drive(replica_fifo.read_data);
            replica_fifo.read_enable.drive(replica_buffer.ingress_read_enable);
            let replica_fifo_read_data_valid = m.reg("replica_fifo_read_data_valid", 1);
            replica_fifo_read_data_valid.default_value(false);
            replica_fifo_read_data_valid.drive_next(!replica_fifo.empty & replica_buffer.ingress_read_enable);
            replica_buffer.ingress_data_valid.drive(replica_fifo_read_data_valid);
            return_arbiter.replica_buffer_egress_ready.unwrap().drive(replica_buffer.egress_ready);
            return_arbiter.replica_buffer_egress_data.unwrap().drive(replica_buffer.egress_data);
            replica_buffer.egress_read_enable.drive(return_arbiter.replica_buffer_egress_read_enable);
        }

        let replica_ports = (0..num_replicas).map(|i| {
            let replica_data_fifo = Fifo::new(format!("replica{}_data_fifo", i), fifo_depth_bits, data_bit_width, m);
            let bus_read_data_valid = m.input(format!("replica{}_bus_read_data_valid", i), 1);
            let bus_read_data = m.input(format!("replica{}_bus_read_data", i), data_bit_width);
            replica_data_fifo.write_enable.drive(bus_read_data_valid);
            replica_data_fifo.write_data.drive(bus_read_data);
            replica_data_fifo.read_enable.drive(return_arbiter.replica_data_fifo_read_enable_outputs[i as usize]);
            return_arbiter.replica_data_fifo_empty_inputs[i as usize].drive(replica_data_fifo.empty);
            return_arbiter.replica_data_fifo_read_data_inputs[i as usize].drive(replica_data_fifo.read_data);
            let replica_issue = &issue.replica_issues[i as usize];
            ReplicaPort {
                bus_enable: replica_issue.bus_enable,
                bus_addr: replica_issue.bus_addr,
                bus_write: replica_issue.bus_write,
                bus_write_data: replica_issue.bus_write_data,
                bus_write_byte_enable: replica_issue.bus_write_byte_enable,
                bus_ready: replica_issue.bus_ready,
                bus_read_data,
                bus_read_data_valid,
            }
        }).collect::<Vec<_>>();

        Crossbar {
            m,
            primary_ports,
            replica_ports,
        }
    }
}

struct PrimaryIssue<'a> {
    name: String,
    bus_enable: &'a Input<'a>,
    bus_addr: &'a Input<'a>,
    bus_write: &'a Input<'a>,
    bus_write_data: &'a Input<'a>,
    bus_write_byte_enable: &'a Input<'a>,
    bus_ready: &'a Output<'a>,
}

struct IssueArbiter<'a> {
    m: &'a Module<'a>,
    primary_issues: Vec<PrimaryIssue<'a>>,
    issue_bus_enable: &'a Output<'a>,
    issue_bus_addr: &'a Output<'a>,
    issue_bus_write: &'a Output<'a>,
    issue_bus_write_data: &'a Output<'a>,
    issue_bus_write_byte_enable: &'a Output<'a>,
    issue_bus_ready: &'a Input<'a>,
    issue_bus_primary: Option<&'a Output<'a>>,
}

impl<'a> IssueArbiter<'a> {
    fn new<S: Into<String>, P: ModuleParent<'a>>(
        instance_name: S,
        num_primaries: u32,
        addr_bit_width: u32,
        data_bit_width: u32,
        data_byte_width: u32,
        primary_select_bit_width: u32,
        p: &'a P,
    ) -> IssueArbiter<'a> {
        let m = p.module(instance_name, "IssueArbiter");

        let issue_bus_ready = m.input("issue_bus_ready", 1);
        let mut bus_ready: &'a Signal<'a> = issue_bus_ready.into();

        let mut primary_issues = Vec::with_capacity(num_primaries as _);
        for i in 0..num_primaries {
            let name = format!("primary{}", i);
            let bus_enable = m.input(format!("{}_bus_enable", name), 1);
            primary_issues.push(PrimaryIssue {
                name: name.clone(),
                bus_enable,
                bus_addr: m.input(format!("{}_bus_addr", name), addr_bit_width),
                bus_write: m.input(format!("{}_bus_write", name), 1),
                bus_write_data: m.input(format!("{}_bus_write_data", name), data_bit_width),
                bus_write_byte_enable: m.input(format!("{}_bus_write_byte_enable", name), data_byte_width),
                bus_ready: m.output(format!("{}_bus_ready", name), bus_ready),
            });
            bus_ready = bus_ready & !bus_enable;
        }
        let primary_issues = primary_issues;

        let last_primary_issue = primary_issues.last().unwrap();
        let mut bus_enable: &'a Signal<'a> = last_primary_issue.bus_enable.into();
        let mut bus_addr: &'a Signal<'a> = last_primary_issue.bus_addr.into();
        let mut bus_write: &'a Signal<'a> = last_primary_issue.bus_write.into();
        let mut bus_write_data: &'a Signal<'a> = last_primary_issue.bus_write_data.into();
        let mut bus_write_byte_enable: &'a Signal<'a> = last_primary_issue.bus_write_byte_enable.into();

        for primary_issue in primary_issues.iter().rev().skip(1) {
            let (new_bus_enable, new_bus_addr, new_bus_write, new_bus_write_data, new_bus_write_byte_enable) = if_(primary_issue.bus_enable, {
                (m.high(), primary_issue.bus_addr, primary_issue.bus_write, primary_issue.bus_write_data, primary_issue.bus_write_byte_enable)
            }).else_({
                (bus_enable, bus_addr, bus_write, bus_write_data, bus_write_byte_enable)
            });
            bus_enable = new_bus_enable;
            bus_addr = new_bus_addr;
            bus_write = new_bus_write;
            bus_write_data = new_bus_write_data;
            bus_write_byte_enable = new_bus_write_byte_enable;
        }

        let issue_bus_enable = m.output("issue_bus_enable", bus_enable);
        let issue_bus_addr = m.output("issue_bus_addr", bus_addr);
        let issue_bus_write = m.output("issue_bus_write", bus_write);
        let issue_bus_write_data = m.output("issue_bus_write_data", bus_write_data);
        let issue_bus_write_byte_enable = m.output("issue_bus_write_byte_enable", bus_write_byte_enable);

        let issue_bus_primary = if num_primaries > 1 {
            let mut bus_primary = m.lit(num_primaries - 1, primary_select_bit_width);

            for (i, primary_issue) in primary_issues.iter().enumerate().rev().skip(1) {
                bus_primary = if_(primary_issue.bus_enable, {
                    m.lit(i as u32, primary_select_bit_width)
                }).else_({
                    bus_primary
                });
            }

            Some(m.output("issue_bus_primary", bus_primary))
        } else {
            None
        };

        IssueArbiter {
            m,
            primary_issues,
            issue_bus_enable,
            issue_bus_addr,
            issue_bus_write,
            issue_bus_write_data,
            issue_bus_write_byte_enable,
            issue_bus_ready,
            issue_bus_primary,
        }
    }
}

struct ReplicaIssue<'a> {
    name: String,
    bus_enable: &'a Output<'a>,
    bus_addr: &'a Output<'a>,
    bus_write: &'a Output<'a>,
    bus_write_data: &'a Output<'a>,
    bus_write_byte_enable: &'a Output<'a>,
    bus_ready: &'a Input<'a>,
}

struct Issue<'a> {
    m: &'a Module<'a>,
    replica_issues: Vec<ReplicaIssue<'a>>,
    issue_arb_bus_enable: &'a Input<'a>,
    issue_arb_bus_addr: &'a Input<'a>,
    issue_arb_bus_write: &'a Input<'a>,
    issue_arb_bus_write_data: &'a Input<'a>,
    issue_arb_bus_write_byte_enable: &'a Input<'a>,
    issue_arb_bus_ready: &'a Output<'a>,
    issue_arb_bus_primary: Option<&'a Input<'a>>,
    primary_fifo_full: Option<&'a Input<'a>>,
    primary_fifo_write_enable: Option<&'a Output<'a>>,
    primary_fifo_write_data: Option<&'a Output<'a>>,
    replica_fifo_full: Option<&'a Input<'a>>,
    replica_fifo_write_enable: Option<&'a Output<'a>>,
    replica_fifo_write_data: Option<&'a Output<'a>>,
}

impl<'a> Issue<'a> {
    fn new<S: Into<String>, P: ModuleParent<'a>>(
        instance_name: S,
        num_primaries: u32,
        num_replicas: u32,
        addr_bit_width: u32,
        replica_select_bit_width: u32,
        data_bit_width: u32,
        data_byte_width: u32,
        primary_select_bit_width: u32,
        replica_addr_bit_width: u32,
        p: &'a P,
    ) -> Issue<'a> {
        let m = p.module(instance_name, "Issue");

        let issue_arb_bus_enable = m.input("issue_arb_bus_enable", 1);
        let issue_arb_bus_addr = m.input("issue_arb_bus_addr", addr_bit_width);
        let issue_arb_bus_write = m.input("issue_arb_bus_write", 1);
        let issue_arb_bus_write_data = m.input("issue_arb_bus_write_data", data_bit_width);
        let issue_arb_bus_write_byte_enable = m.input("issue_arb_bus_write_byte_enable", data_byte_width);

        let primary_fifo_full = if num_primaries > 1 { Some(m.input("primary_fifo_full", 1)) } else { None };
        let primary_fifo_write_ready = !primary_fifo_full.map(|x| x.into()).unwrap_or(m.low());
        let replica_fifo_full = if num_replicas > 1 { Some(m.input("replica_fifo_full", 1)) } else { None };
        let replica_fifo_write_ready = !replica_fifo_full.map(|x| x.into()).unwrap_or(m.low());

        let buster_issue_ready = issue_arb_bus_write | (primary_fifo_write_ready & replica_fifo_write_ready);

        let replica_bus_enable = issue_arb_bus_enable & buster_issue_ready;

        let mut replica_bus_ready = m.low();
        let replica_select = if num_replicas > 1 {
            Some(issue_arb_bus_addr.bits(addr_bit_width - 1, replica_addr_bit_width))
        } else {
            None
        };

        let mut replica_issues = Vec::with_capacity(num_replicas as _);
        for i in 0..num_replicas {
            let name = format!("replica{}", i);
            let bus_ready = m.input(format!("{}_bus_ready", name), 1);

            let local_replica_select = replica_select.map(|x| x.eq(m.lit(i, replica_select_bit_width))).unwrap_or(m.high());
            replica_bus_ready = replica_bus_ready | (if replica_select.is_some() { bus_ready.into() } else { m.high() } & local_replica_select);

            replica_issues.push(ReplicaIssue {
                name: name.clone(),
                bus_enable: m.output(format!("{}_bus_enable", name), replica_bus_enable & local_replica_select),
                bus_addr: m.output(format!("{}_bus_addr", name), issue_arb_bus_addr.bits(replica_addr_bit_width - 1, 0)),
                bus_write: m.output(format!("{}_bus_write", name), issue_arb_bus_write),
                bus_write_data: m.output(format!("{}_bus_write_data", name), issue_arb_bus_write_data),
                bus_write_byte_enable: m.output(format!("{}_bus_write_byte_enable", name), issue_arb_bus_write_byte_enable),
                bus_ready,
            });
        }
        let replica_issues = replica_issues;

        let (issue_arb_bus_primary, primary_fifo_write_enable, primary_fifo_write_data) = if num_primaries > 1 {
            let issue_arb_bus_primary = m.input("issue_arb_bus_primary", primary_select_bit_width);
            let primary_fifo_write_enable = m.output("primary_fifo_write_enable", issue_arb_bus_enable & !issue_arb_bus_write & buster_issue_ready & replica_bus_ready);
            let primary_fifo_write_data = m.output("primary_fifo_write_data", issue_arb_bus_primary);
            (Some(issue_arb_bus_primary), Some(primary_fifo_write_enable), Some(primary_fifo_write_data))
        } else {
            (None, None, None)
        };

        let (replica_fifo_write_enable, replica_fifo_write_data) = if num_replicas > 1 {
            let replica_fifo_write_enable = m.output("replica_fifo_write_enable", issue_arb_bus_enable & !issue_arb_bus_write & buster_issue_ready & replica_bus_ready);
            let replica_fifo_write_data = m.output("replica_fifo_write_data", replica_select.unwrap());
            (Some(replica_fifo_write_enable), Some(replica_fifo_write_data))
        } else {
            (None, None)
        };

        let issue_arb_bus_ready = m.output("issue_arb_bus_ready", buster_issue_ready & replica_bus_ready);

        Issue {
            m,
            replica_issues,
            issue_arb_bus_enable,
            issue_arb_bus_addr,
            issue_arb_bus_write,
            issue_arb_bus_write_data,
            issue_arb_bus_write_byte_enable,
            issue_arb_bus_ready,
            issue_arb_bus_primary,
            primary_fifo_full,
            primary_fifo_write_enable,
            primary_fifo_write_data,
            replica_fifo_full,
            replica_fifo_write_enable,
            replica_fifo_write_data,
        }
    }
}

struct ReturnArbiter<'a> {
    pub m: &'a Module<'a>,
    primary_fifo_empty: Option<&'a Input<'a>>,
    replica_data_fifo_empty_inputs: Vec<&'a Input<'a>>,
    replica_data_fifo_read_data_inputs: Vec<&'a Input<'a>>,
    replica_buffer_egress_ready: Option<&'a Input<'a>>,
    replica_buffer_egress_data: Option<&'a Input<'a>>,
    primary_fifo_read_enable: &'a Output<'a>,
    replica_buffer_egress_read_enable: &'a Output<'a>,
    replica_data_fifo_read_enable_outputs: Vec<&'a Output<'a>>,
    primary_fifo_read_data: Option<&'a Input<'a>>,
    primary_bus_read_data_outputs: Vec<&'a Output<'a>>,
    primary_bus_read_data_valid_outputs: Vec<&'a Output<'a>>,
}

impl<'a> ReturnArbiter<'a> {
    fn new<S: Into<String>, P: ModuleParent<'a>>(
        instance_name: S,
        num_primaries: u32,
        num_replicas: u32,
        replica_select_bit_width: u32,
        data_bit_width: u32,
        primary_select_bit_width: u32,
        p: &'a P,
    ) -> ReturnArbiter<'a> {
        let m = p.module(instance_name, "ReturnArbiter");

        let primary_fifo_empty = if num_primaries > 1 { Some(m.input("primary_fifo_empty", 1)) } else { None };
        let primary_fifo_read_ready = !primary_fifo_empty.map(|x| x.into()).unwrap_or(m.low());

        let replica_data_fifo_empty_inputs = (0..num_replicas).map(|i| m.input(format!("replica{}_data_fifo_empty", i), 1)).collect::<Vec<_>>();
        let replica_data_fifo_read_data_inputs = (0..num_replicas).map(|i| m.input(format!("replica{}_data_fifo_read_data", i), data_bit_width)).collect::<Vec<_>>();

        let mut replica_data_fifo_read_ready = !replica_data_fifo_empty_inputs[(num_replicas - 1) as usize];
        let mut replica_data = replica_data_fifo_read_data_inputs[(num_replicas - 1) as usize].into();
        let (replica_buffer_egress_ready, replica_buffer_egress_data) = if num_replicas > 1 {
            let replica_buffer_egress_ready = m.input("replica_buffer_egress_ready", 1);
            let replica_buffer_egress_data = m.input("replica_buffer_egress_data", replica_select_bit_width);

            let replica_data_fifo_select = m.reg("replica_data_fifo_select", replica_select_bit_width);
            replica_data_fifo_select.drive_next(replica_buffer_egress_data);

            for i in (0..num_replicas).rev().skip(1) {
                replica_data_fifo_read_ready = if_(replica_buffer_egress_data.eq(m.lit(i, replica_select_bit_width)), {
                    let replica_data_fifo_empty = replica_data_fifo_empty_inputs[i as usize];
                    let replica_data_fifo_read_ready = !replica_data_fifo_empty;
                    replica_data_fifo_read_ready
                }).else_({
                    replica_data_fifo_read_ready
                });
                replica_data = if_(replica_data_fifo_select.eq(m.lit(i, replica_select_bit_width)), {
                    let replica_data_fifo_read_data = replica_data_fifo_read_data_inputs[i as usize];
                    replica_data_fifo_read_data
                }).else_({
                    replica_data
                });
            }

            (Some(replica_buffer_egress_ready), Some(replica_buffer_egress_data))
        } else {
            (None, None)
        };

        let fifo_read_enable = primary_fifo_read_ready & replica_buffer_egress_ready.map(|x| x.into()).unwrap_or(m.high()) & replica_data_fifo_read_ready;
        let primary_fifo_read_enable = m.output("primary_fifo_read_enable", fifo_read_enable);
        let replica_buffer_egress_read_enable = m.output("replica_buffer_egress_read_enable", fifo_read_enable);
        let replica_data_fifo_read_enable_outputs = (0..num_replicas).map(|i| m.output(
            format!("replica{}_data_fifo_read_enable", i),
            fifo_read_enable & replica_buffer_egress_data.map(|x| x.eq(m.lit(i, replica_select_bit_width))).unwrap_or(m.high()),
        )).collect::<Vec<_>>();

        let fifo_read_data_valid = m.reg("fifo_read_data_valid", 1);
        fifo_read_data_valid.default_value(false);
        fifo_read_data_valid.drive_next(fifo_read_enable);

        let primary_fifo_read_data = if num_primaries > 1 {
            Some(m.input("primary_fifo_read_data", primary_select_bit_width))
        } else {
            None
        };
        let primary_bus_read_data_outputs = (0..num_primaries).map(|i| m.output(format!("primary{}_bus_read_data", i), replica_data)).collect::<Vec<_>>();
        let primary_bus_read_data_valid_outputs = (0..num_primaries).map(|i| {
            let primary_fifo_read_data = primary_fifo_read_data.map(|x| x.eq(m.lit(i as u32, primary_select_bit_width))).unwrap_or(m.high());
            m.output(format!("primary{}_bus_read_data_valid", i), fifo_read_data_valid & primary_fifo_read_data)
        }).collect::<Vec<_>>();

        ReturnArbiter {
            m,
            primary_fifo_empty,
            replica_data_fifo_empty_inputs,
            replica_data_fifo_read_data_inputs,
            replica_buffer_egress_ready,
            replica_buffer_egress_data,
            primary_fifo_read_enable,
            replica_buffer_egress_read_enable,
            replica_data_fifo_read_enable_outputs,
            primary_fifo_read_data,
            primary_bus_read_data_outputs,
            primary_bus_read_data_valid_outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Cannot generate a buster crossbar module with zero primaries.")]
    fn zero_primaries_error() {
        let c = Context::new();

        // Panic
        let _ = Crossbar::new("bad_duder", 0, 2, 2, 1, 1, 1, &c);
    }

    #[test]
    #[should_panic(expected = "Cannot generate a buster crossbar module with zero replicas.")]
    fn zero_replicas_error() {
        let c = Context::new();

        // Panic
        let _ = Crossbar::new("bad_duder", 2, 0, 2, 1, 1, 1, &c);
    }
}
