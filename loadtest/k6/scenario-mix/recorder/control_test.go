package recorder

import (
	"context"
	"testing"
	"time"
)

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
