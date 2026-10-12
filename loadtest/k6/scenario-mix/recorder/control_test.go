package recorder

import (
	"context"
	"math"
	"testing"
	"time"
)

func TestRejectedFailoverSkipsToNextStageWithoutCatchupTraffic(t *testing.T) {
	now := time.Now()
	c := &loadControl{load: controlLoad{Duration: 80, Stages: []controlStage{{Target: 5, Seconds: 60}, {Target: 10, Seconds: 20}}}, flows: map[string]*controlFlow{}, state: "running", active: 20, started: now, last: now}
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	for _, invalid := range []float64{19, 81, math.NaN(), math.Inf(1)} {
		if err := c.action("recover", &invalid); err == nil {
			t.Fatal("invalid skip boundary accepted")
		}
	}
	end := 60.0
	if err := c.action("recover", &end); err != nil {
		t.Fatal(err)
	}
	if c.active != 60 || c.faultGate || c.faultCycle {
		t.Fatal("did not move directly to next stage")
	}
	if c.inFlight != 0 || c.dropped != 0 {
		t.Fatal("skipping manufactured traffic")
	}
}

func TestControlPauseDrainsAndFreezesAdmission(t *testing.T) {
	now := time.Now()
	f := &controlFlow{Weight: 100, Requests: make(chan admission, 10)}
	c := &loadControl{load: controlLoad{Rate: 10, Duration: 3}, flows: map[string]*controlFlow{"guest": f}, state: "running", started: now, last: now}
	request := admission{ctx: context.Background(), ready: make(chan bool, 1)}
	f.Requests <- request
	c.step(now.Add(110 * time.Millisecond))
	if c.inFlight != 1 {
		t.Fatal("did not admit flow")
	}
	if s := c.snapshot(); s.Started != 1 || s.FlowStarts["guest"] != 1 {
		t.Fatal("started counters must include admitted flows before responses return")
	}
	if err := c.action("pause"); err != nil {
		t.Fatal(err)
	}
	if c.state != "pausing" {
		t.Fatal("must drain before paused")
	}
	if err := c.action("resume"); err == nil {
		t.Fatal("resume while pausing allowed")
	}
	credit := f.Credit
	active := c.active
	c.step(now.Add(time.Second))
	if c.active != active {
		t.Fatal("paused clock advanced")
	}
	c.leave()
	if c.state != "paused" || c.inFlight != 0 {
		t.Fatal("did not reach paused")
	}
	if err := c.action("resume"); err != nil {
		t.Fatal(err)
	}
	if f.Credit != credit {
		t.Fatal("resume discarded already accrued fractional credit")
	}
	if c.state != "running" {
		t.Fatal("did not resume")
	}
	c.active = 3
	c.state = "draining"
	if !c.step(time.Now()) || c.state != "completed" {
		t.Fatal("did not complete after drain")
	}
}

func TestControlDropsUnavailableSlotsAndPreservesRampClock(t *testing.T) {
	now := time.Now()
	c := &loadControl{load: controlLoad{Start: 0, Duration: 2, Stages: []controlStage{{Target: 10, Seconds: 2}}}, flows: map[string]*controlFlow{"guest": {Weight: 100, Requests: make(chan admission, 1)}}, state: "running", started: now, last: now}
	c.step(now.Add(time.Second))
	if c.rate() != 5 || c.dropped != 2 {
		t.Fatal("ramp or dropped admission counters incorrect")
	}
	if c.snapshot().Started != 0 {
		t.Fatal("dropped admissions must not count as started flows")
	}
	active := c.active
	c.step(now.Add(-time.Second))
	if c.active != active {
		t.Fatal("stale ticker moved active clock backwards")
	}
	if err := c.action("pause"); err != nil {
		t.Fatal(err)
	}
	if err := c.action("pause"); err == nil {
		t.Fatal("second pause accepted")
	}
	c.step(now.Add(10 * time.Second))
	if c.active != active {
		t.Fatal("pause consumed ramp duration")
	}
}

func TestControlFractionalWeightsDeliverWholeFinalSlots(t *testing.T) {
	now := time.Now()
	flows := map[string]*controlFlow{}
	for name, weight := range map[string]float64{"cit": 50, "sdk": 30, "guest": 20} {
		f := &controlFlow{Weight: weight, Requests: make(chan admission, 100)}
		for i := 0; i < 100; i++ {
			f.Requests <- admission{ctx: context.Background(), ready: make(chan bool, 1)}
		}
		flows[name] = f
	}
	c := &loadControl{load: controlLoad{Rate: 10, Duration: 3}, flows: flows, state: "running", started: now, last: now}
	for i := 1; i <= 3000; i++ {
		c.step(now.Add(time.Duration(i) * time.Millisecond))
		c.inFlight = 0
	}
	for name, want := range map[string]int{"cit": 15, "sdk": 9, "guest": 6} {
		if got := 100 - len(flows[name].Requests); got != want {
			t.Fatalf("%s: got %d, want %d", name, got, want)
		}
	}
	if c.dropped != 0 {
		t.Fatal("unexpected drops")
	}
}

func TestFaultGateFreezesStageClockButKeepsTrafficFlowing(t *testing.T) {
	now := time.Now()
	f := &controlFlow{Weight: 100, Requests: make(chan admission, 100)}
	for i := 0; i < 100; i++ {
		f.Requests <- admission{ctx: context.Background(), ready: make(chan bool, 1)}
	}
	c := &loadControl{load: controlLoad{Rate: 10, Duration: 30}, flows: map[string]*controlFlow{"guest": f}, state: "running", started: now, last: now}
	c.step(now.Add(time.Second))
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	gateStarted := c.last
	active := c.active
	c.step(gateStarted.Add(2 * time.Second))
	if c.active != active {
		t.Fatal("fault setup consumed configured stage time")
	}
	if got := 100 - len(f.Requests); got != 30 {
		t.Fatalf("traffic stopped during fault gate: admitted %d, want 30", got)
	}
	if snapshot := c.snapshot(); !snapshot.FaultGate || snapshot.State != "running" || snapshot.Offered != 10 {
		t.Fatalf("unexpected gated snapshot: %+v", snapshot)
	}
	if err := c.action("release"); err != nil {
		t.Fatal(err)
	}
	released := c.last
	c.step(released.Add(3 * time.Second))
	if c.active != active+3 {
		t.Fatalf("confirmed outage did not consume configured time: got %v", c.active)
	}
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	recoveryStarted := c.last
	c.step(recoveryStarted.Add(2 * time.Second))
	if c.active != active+3 {
		t.Fatal("recovery consumed time before the next stage")
	}
	if err := c.action("recover"); err != nil {
		t.Fatal(err)
	}
}

func TestFinalFaultStageCannotCompleteBeforeVerifiedRecovery(t *testing.T) {
	now := time.Now()
	c := &loadControl{load: controlLoad{Rate: 1, Duration: 1}, flows: map[string]*controlFlow{}, state: "running", started: now, last: now}
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	if err := c.action("release"); err != nil {
		t.Fatal(err)
	}
	released := c.last
	c.step(released.Add(time.Second))
	if c.active != 1 || c.state != "running" {
		t.Fatalf("final fault stage completed before recovery: active=%v state=%s", c.active, c.state)
	}
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	if err := c.action("recover"); err != nil {
		t.Fatal(err)
	}
	if !c.step(c.last.Add(time.Millisecond)) || c.state != "completed" {
		t.Fatalf("load did not complete after verified recovery: %s", c.state)
	}
}

func TestRecoveryGatePreventsTheNextStageFromStartingEarly(t *testing.T) {
	now := time.Now()
	c := &loadControl{load: controlLoad{Start: 1, Duration: 30, Stages: []controlStage{{Target: 1, Seconds: 20}, {Target: 5, Seconds: 10}}}, flows: map[string]*controlFlow{}, state: "running", started: now, last: now}
	c.step(now.Add(10 * time.Second))
	if err := c.action("gate"); err != nil {
		t.Fatal(err)
	}
	boundary := 20.0
	invalid := 31.0
	if err := c.action("release", &invalid); err == nil || !c.faultGate {
		t.Fatal("invalid release must preserve the fault gate")
	}
	if err := c.action("release", &boundary); err != nil {
		t.Fatal(err)
	}
	released := c.last
	c.step(released.Add(15 * time.Second))
	if c.active != boundary || !c.faultGate {
		t.Fatalf("load passed the fault-stage boundary before recovery: active=%v faultGate=%v", c.active, c.faultGate)
	}
	if got := c.rate(); got != 1 {
		t.Fatalf("next ramp started before recovery verification: rate=%v", got)
	}
	if err := c.action("recover"); err != nil {
		t.Fatal(err)
	}
	recovered := c.last
	c.step(recovered.Add(time.Second))
	if c.active != boundary+1 || c.rate() <= 1 {
		t.Fatalf("next ramp did not begin after recovery: active=%v rate=%v", c.active, c.rate())
	}
}
