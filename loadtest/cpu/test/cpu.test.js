"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const {
  allocateSelectedThreads,
  allocateSelectedWorkloads,
  recommendAllocation,
  validateAllocation,
} = require("../lib/allocation");
const { currentManagedOptions } = require("../lib/artifacts");
const { formatCpuList, parseLscpuCsv } = require("../lib/topology");
const { gridRows, gridScreen } = require("../lib/tui");

const smtTopology = parseLscpuCsv(`# CPU,Core,Socket,Node,Online
0,0,0,0,Y
1,1,0,0,Y
2,2,0,0,Y
3,3,0,0,Y
4,4,0,0,Y
5,5,0,0,Y
6,6,0,0,Y
7,7,0,0,Y
16,0,0,0,Y
17,1,0,0,Y
18,2,0,0,Y
19,3,0,0,Y
20,4,0,0,Y
21,5,0,0,Y
22,6,0,0,Y
23,7,0,0,Y`);

const config = {
  application_services: {
    router: { enabled: true },
    "modular-pm": { enabled: true },
    encryption: { enabled: true },
    vault: { enabled: true },
  },
  state_services: { postgres: { enabled: true }, redis: { enabled: true } },
};

test("discovers physical cores and non-contiguous SMT siblings", () => {
  assert.equal(smtTopology.physical_core_count, 8);
  assert.equal(smtTopology.logical_cpu_count, 16);
  assert.deepEqual(smtTopology.cores[3].logical_cpus, [3, 19]);
});

test("allocates host cores, dedicated workloads, sibling state, and k6", () => {
  const allocation = recommendAllocation(smtTopology, config, 2);
  assert.equal(allocation.host_logical_cpus, "0-1,16-17");
  assert.equal(allocation.assignments.router, "2");
  assert.equal(allocation.assignments.postgres, "6");
  assert.equal(allocation.assignments.redis, "22");
  assert.equal(allocation.assignments.k6, "7");
  assert.deepEqual(validateAllocation(smtTopology, allocation), []);
});

test("shares state logical CPU when SMT is unavailable", () => {
  const source = Array.from({ length: 8 }, (_, cpu) => `${cpu},${cpu},0,0,Y`).join("\n");
  const topology = parseLscpuCsv(source);
  const allocation = recommendAllocation(topology, config, 2);
  assert.equal(allocation.assignments.postgres, allocation.assignments.redis);
  assert.deepEqual(validateAllocation(topology, allocation), []);
});

test("assigns every physical core not selected for a workload to the host", () => {
  const allocation = allocateSelectedWorkloads(smtTopology, config, {
    router: "socket0/core0",
    "modular-pm": "socket0/core2",
    encryption: "socket0/core4",
    vault: "socket0/core5",
    state: "socket0/core6",
    k6: "socket0/core7",
  });
  assert.deepEqual(allocation.host_physical_cores, ["socket0/core1", "socket0/core3"]);
  assert.equal(allocation.host_logical_cpus, "1,3,17,19");
  assert.equal(allocation.assignments.router, "0");
  assert.equal(allocation.assignments.postgres, "6");
  assert.equal(allocation.assignments.redis, "22");
  assert.deepEqual(validateAllocation(smtTopology, allocation), []);
});

test("uses explicitly selected logical threads as Docker cpusets", () => {
  const cores = {
    router: "socket0/core0",
    "modular-pm": "socket0/core2",
    encryption: "socket0/core4",
    vault: "socket0/core5",
    state: "socket0/core6",
    k6: "socket0/core7",
  };
  const allocation = allocateSelectedWorkloads(smtTopology, config, cores, {
    router: 16,
    "modular-pm": 18,
    encryption: 20,
    vault: 21,
    postgres: 22,
    redis: 6,
    k6: 23,
  });
  assert.equal(allocation.assignments.router, "16");
  assert.equal(allocation.assignments.postgres, "22");
  assert.equal(allocation.assignments.redis, "6");
  assert.equal(allocation.assignments.k6, "23");
  assert.deepEqual(validateAllocation(smtTopology, allocation), []);
});

test("rejects a logical thread outside its selected physical core", () => {
  assert.throws(() => recommendAllocation(smtTopology, config, 2, {
    workloadLogicalCpus: { router: 0 },
  }), /router logical CPU 0 does not belong/);
});

test("renders sibling threads on the same physical-core row", () => {
  const rows = gridRows(smtTopology.cores.slice(0, 2), 16, new Map([["socket0/core1", "router"]]));
  assert.equal(rows[0], "socket0/core0   ( ) thread0/CPU0  (*) thread1/CPU16");
  assert.equal(rows[1], "socket0/core1   [-] unavailable:router");
});

test("renders the thread grid as a bordered TUI panel", () => {
  const screen = gridScreen({
    title: "Assign router",
    prompt: "16 cores / 32 CPUs",
    rows: ["socket0/core0   ( ) thread0/CPU0  (*) thread1/CPU16"],
    cursorRow: 0,
    width: 78,
  });
  assert.match(screen, /^\+-+\+\n\| CPU allocation \| Assign router/);
  assert.match(screen, /\|> socket0\/core0   \( \) thread0\/CPU0  \(\*\) thread1\/CPU16/);
  assert.match(screen, /Left\/Right: thread/);
});

test("locks only selected threads and assigns every remaining thread to the host", () => {
  const allocation = allocateSelectedThreads(smtTopology, config, {
    router: 0,
    "modular-pm": 16,
    encryption: 1,
    vault: 17,
    postgres: 2,
    redis: 18,
    k6: 3,
  });
  assert.equal(allocation.isolated_logical_cpus, "0-3,16-18");
  assert.equal(allocation.host_logical_cpus, "4-7,19-23");
  assert.equal(allocation.assignments.router, "0");
  assert.equal(allocation.assignments["modular-pm"], "16");
  assert.equal(allocation.reserved_logical_cpus, "");
  assert.deepEqual(validateAllocation(smtTopology, allocation), []);
});

test("requires one physical core to remain for the host", () => {
  const source = Array.from({ length: 6 }, (_, cpu) => `${cpu},${cpu},0,0,Y`).join("\n");
  const topology = parseLscpuCsv(source);
  assert.throws(() => allocateSelectedWorkloads(topology, config, {
    router: "socket0/core0",
    "modular-pm": "socket0/core1",
    encryption: "socket0/core2",
    vault: "socket0/core3",
    state: "socket0/core4",
    k6: "socket0/core5",
  }), /At least one physical core must remain/);
});

test("rejects insufficient capacity and host overlap", () => {
  assert.throws(() => recommendAllocation(smtTopology, config, 3), /Need 3 host \+ 6 workload/);
  const allocation = recommendAllocation(smtTopology, config, 2);
  allocation.assignments.router = "0";
  assert.match(validateAllocation(smtTopology, allocation).join(";"), /overlaps the host set/);
});

test("formats CPU ranges and isolates managed kernel options", () => {
  assert.equal(formatCpuList([0, 1, 4, 6, 5]), "0-1,4-6");
  assert.deepEqual(
    currentManagedOptions("quiet splash isolcpus=2-7 nohz_full=2-7 unrelated=yes"),
    ["isolcpus=2-7", "nohz_full=2-7"],
  );
});
