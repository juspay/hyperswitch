package recorder

import (
	"context"
	"crypto/subtle"
	"encoding/json"
	"errors"
	"math"
	"net"
	"net/http"
	"os"
	"sync"
	"time"
)

type controlStage struct {
	Target  float64 `json:"target_rps"`
	Seconds float64 `json:"duration_seconds"`
}
type controlLoad struct {
	Rate     float64        `json:"total_rps"`
	Duration float64        `json:"duration_seconds"`
	Start    float64        `json:"start_rps"`
	Stages   []controlStage `json:"stages"`
}
type admission struct {
	ctx   context.Context
	ready chan bool
}
type controlFlow struct {
	Weight   float64
	Requests chan admission
	Credit   float64
	Started  uint64
}
type controlSnapshot struct {
	State      string            `json:"state"`
	FaultGate  bool              `json:"fault_gate"`
	Active     float64           `json:"active_seconds"`
	Wall       float64           `json:"wall_seconds"`
	InFlight   uint64            `json:"in_flight"`
	Dropped    uint64            `json:"dropped"`
	Offered    float64           `json:"offered_tps"`
	Started    uint64            `json:"started_flows"`
	FlowStarts map[string]uint64 `json:"flow_starts"`
}
type loadControl struct {
	mu                sync.Mutex
	load              controlLoad
	flows             map[string]*controlFlow
	state             string
	started, last     time.Time
	active            float64
	faultGate         bool
	faultCycle        bool
	gateAt            float64
	gateRate          float64
	inFlight, dropped uint64
	stop              chan struct{}
	done              chan struct{}
	server            *http.Server
	file              string
	onDone            func()
}

func newLoadControl(metadata map[string]interface{}, file, token string, onDone func()) (*loadControl, error) {
	if token == "" {
		return nil, errors.New("console control token is required")
	}
	raw, _ := json.Marshal(metadata["load"])
	var load controlLoad
	if err := json.Unmarshal(raw, &load); err != nil {
		return nil, err
	}
	if load.Start < 0 {
		return nil, errors.New("console start rate must be nonnegative")
	}
	if len(load.Stages) > 0 {
		load.Duration = 0
		for _, s := range load.Stages {
			if s.Target <= 0 || s.Seconds <= 0 {
				return nil, errors.New("invalid console stage")
			}
			load.Duration += s.Seconds
		}
		var loadFields map[string]interface{}
		_ = json.Unmarshal(raw, &loadFields)
		if _, explicitStart := loadFields["start_rps"]; !explicitStart {
			load.Start = load.Stages[0].Target
		}
	} else if load.Rate <= 0 || load.Duration <= 0 {
		return nil, errors.New("console control requires positive rate and duration")
	}
	c := &loadControl{load: load, flows: map[string]*controlFlow{}, state: "idle", file: file, stop: make(chan struct{}), done: make(chan struct{}), onDone: onDone}
	if entries, ok := metadata["scenarios"].([]interface{}); ok {
		for _, raw := range entries {
			entry := raw.(map[string]interface{})
			name := entry["name"].(string)
			weight := entry["weight"].(float64)
			if weight > 0 {
				c.flows[name] = &controlFlow{Weight: weight, Requests: make(chan admission, 100000)}
			}
		}
	}
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return nil, err
	}
	c.server = &http.Server{ReadHeaderTimeout: 5 * time.Second, Handler: http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if subtle.ConstantTimeCompare([]byte(r.Header.Get("Authorization")), []byte("Bearer "+token)) != 1 {
			http.Error(w, "unauthorized", 401)
			return
		}
		if r.URL.Path != "/control" {
			http.NotFound(w, r)
			return
		}
		if r.Method == "POST" {
			var body struct {
				Action             string   `json:"action"`
				UntilActiveSeconds *float64 `json:"until_active_seconds"`
			}
			if json.NewDecoder(http.MaxBytesReader(w, r.Body, 1024)).Decode(&body) != nil {
				http.Error(w, "invalid action", 400)
				return
			}
			if err := c.action(body.Action, body.UntilActiveSeconds); err != nil {
				http.Error(w, err.Error(), 409)
				return
			}
		} else if r.Method != "GET" {
			http.Error(w, "method not allowed", 405)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(c.snapshot())
	})}
	if err := c.writeFile(map[string]interface{}{"endpoint": "http://" + listener.Addr().String() + "/control", "state": "idle"}); err != nil {
		listener.Close()
		return nil, err
	}
	go func() { _ = c.server.Serve(listener) }()
	go c.loop()
	return c, nil
}
func (c *loadControl) writeFile(value interface{}) error {
	raw, _ := json.Marshal(value)
	if err := os.WriteFile(c.file+".tmp", raw, 0600); err != nil {
		return err
	}
	return os.Rename(c.file+".tmp", c.file)
}
func (c *loadControl) rate() float64 {
	if len(c.load.Stages) == 0 {
		return c.load.Rate
	}
	t := c.active
	start := c.load.Start
	for _, s := range c.load.Stages {
		if t <= s.Seconds {
			return start + (s.Target-start)*t/s.Seconds
		}
		t -= s.Seconds
		start = s.Target
	}
	return start
}

// Integrate each linear ramp exactly, even when a scheduling tick is delayed.
func (c *loadControl) offeredBetween(from, to float64) float64 {
	if len(c.load.Stages) == 0 {
		return c.load.Rate * (to - from)
	}
	area, offset, start := 0.0, 0.0, c.load.Start
	for _, stage := range c.load.Stages {
		a, b := math.Max(from, offset), math.Min(to, offset+stage.Seconds)
		if b > a {
			first := start + (stage.Target-start)*(a-offset)/stage.Seconds
			last := start + (stage.Target-start)*(b-offset)/stage.Seconds
			area += (first + last) / 2 * (b - a)
		}
		offset += stage.Seconds
		start = stage.Target
		if offset >= to {
			break
		}
	}
	return area
}
func (c *loadControl) snapshot() controlSnapshot {
	c.mu.Lock()
	defer c.mu.Unlock()
	wall := 0.0
	if !c.started.IsZero() {
		wall = time.Since(c.started).Seconds()
	}
	rate := c.rate()
	if c.state != "running" {
		rate = 0
	}
	starts, total := make(map[string]uint64, len(c.flows)), uint64(0)
	for name, flow := range c.flows {
		starts[name] = flow.Started
		total += flow.Started
	}
	return controlSnapshot{State: c.state, FaultGate: c.faultGate, Active: c.active, Wall: wall, InFlight: c.inFlight, Dropped: c.dropped, Offered: rate, Started: total, FlowStarts: starts}
}
func (c *loadControl) action(action string, until ...*float64) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	var gateAt *float64
	if len(until) > 0 {
		gateAt = until[0]
	}
	switch action {
	case "gate":
		if c.state != "running" {
			return errors.New("only a running load can enter a fault gate")
		}
		if !c.faultGate {
			c.gateRate = c.rate()
			c.faultGate = true
			c.faultCycle = true
			c.gateAt = 0
			c.last = time.Now()
		}
	case "release":
		if c.state != "running" {
			return errors.New("only a running load can be released from a fault gate")
		}
		if c.faultGate {
			if gateAt != nil {
				if *gateAt <= c.active || *gateAt > c.load.Duration {
					return errors.New("release boundary must be after the active clock and within the load duration")
				}
				c.gateAt = *gateAt
			} else {
				c.gateAt = 0
			}
			c.faultGate = false
			c.last = time.Now()
		}
	case "recover":
		if c.state != "running" {
			return errors.New("only a running load can finish a fault cycle")
		}
		if gateAt != nil {
			if math.IsNaN(*gateAt) || math.IsInf(*gateAt, 0) || *gateAt < c.active || *gateAt > c.load.Duration {
				return errors.New("recovery boundary must be at or after the active clock and within the load duration")
			}
			c.active = *gateAt
		}
		c.faultGate = false
		c.faultCycle = false
		c.gateAt = 0
		c.last = time.Now()
	case "pause":
		if c.state != "running" || c.faultGate {
			return errors.New("only running loads can pause")
		}
		c.state = "pausing"
		if c.inFlight == 0 {
			c.state = "paused"
		}
	case "resume":
		if c.state != "paused" {
			return errors.New("only paused loads can resume")
		}
		c.state = "running"
		c.last = time.Now()
	default:
		return errors.New("unknown control action")
	}
	return nil
}
func (c *loadControl) enter(ctx context.Context, name string) bool {
	c.mu.Lock()
	f := c.flows[name]
	if c.state == "idle" {
		c.started = time.Now()
		c.last = c.started
		c.state = "running"
	}
	c.mu.Unlock()
	if f == nil {
		return false
	}
	req := admission{ctx: ctx, ready: make(chan bool, 1)}
	select {
	case f.Requests <- req:
	case <-ctx.Done():
		return false
	case <-c.stop:
		return false
	}
	select {
	case ok := <-req.ready:
		return ok
	case <-ctx.Done():
		return false
	case <-c.stop:
		return false
	}
}
func (c *loadControl) leave() {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.inFlight > 0 {
		c.inFlight--
	}
	if c.state == "pausing" && c.inFlight == 0 {
		c.state = "paused"
	}
}
func (c *loadControl) step(now time.Time) bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.state == "running" {
		delta := now.Sub(c.last).Seconds()
		if delta < 0 {
			return false
		}
		c.last = now
		offered := c.gateRate * delta
		if !c.faultGate {
			remaining := c.load.Duration - c.active
			if c.gateAt > c.active && c.gateAt-c.active < remaining {
				remaining = c.gateAt - c.active
			}
			if delta > remaining {
				delta = remaining
			}
			offered = c.offeredBetween(c.active, c.active+delta)
			c.active += delta
			if c.gateAt > 0 && c.active >= c.gateAt-1e-9 {
				c.active = c.gateAt
				c.gateRate = c.rate()
				c.gateAt = 0
				c.faultGate = true
				c.faultCycle = true
			}
		}
		for _, f := range c.flows {
			f.Credit += offered * f.Weight / 100
			slots := int(math.Floor(f.Credit + 1e-9))
			f.Credit -= float64(slots)
			if f.Credit < 0 && f.Credit > -1e-9 {
				f.Credit = 0
			}
			for i := 0; i < slots; i++ {
				delivered := false
				for !delivered {
					select {
					case req := <-f.Requests:
						if req.ctx.Err() != nil {
							continue
						}
						c.inFlight++
						f.Started++
						req.ready <- true
						delivered = true
					default:
						c.dropped++
						delivered = true
					}
				}
			}
		}
		if !c.faultGate && !c.faultCycle && c.active >= c.load.Duration {
			c.state = "draining"
		}
	}
	if c.state == "draining" && c.inFlight == 0 {
		c.state = "completed"
		return true
	}
	return false
}
func (c *loadControl) loop() {
	defer close(c.done)
	ticker := time.NewTicker(time.Millisecond)
	defer ticker.Stop()
	for {
		select {
		case now := <-ticker.C:
			if c.step(now) {
				_ = c.writeFile(c.snapshot())
				c.onDone()
				return
			}
		case <-c.stop:
			return
		}
	}
}
func (c *loadControl) close() {
	close(c.stop)
	<-c.done
	_ = c.server.Close()
	_ = c.writeFile(c.snapshot())
}
