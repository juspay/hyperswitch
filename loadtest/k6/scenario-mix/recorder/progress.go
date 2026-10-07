package recorder

import (
	"encoding/json"
	"os"
	"sort"
	"strings"
	"sync"
	"time"

	"go.k6.io/k6/metrics"
)

// Live telemetry contains counters and one bounded latency window, never responses.
type flowProgress struct {
	Successes     uint64   `json:"successes"`
	Failures      uint64   `json:"failures"`
	TPS           float64  `json:"tps"`
	P90           *float64 `json:"p90_ms"`
	windowSuccess uint64
	latencies     []float64
}
type liveProgress struct {
	Elapsed       float64                  `json:"elapsed_seconds"`
	Iterations    uint64                   `json:"iterations"`
	Dropped       uint64                   `json:"dropped"`
	HTTPAttempts  uint64                   `json:"http_attempts"`
	HTTPErrors    uint64                   `json:"http_errors"`
	SuccessfulTPS float64                  `json:"successful_tps"`
	Recorder      Stats                    `json:"recorder"`
	Flows         map[string]*flowProgress `json:"flows"`
}
type progressCollector struct {
	mu                sync.Mutex
	started, previous time.Time
	state             liveProgress
}

func newProgressCollector(metadata map[string]interface{}) *progressCollector {
	p := &progressCollector{started: time.Now(), previous: time.Now()}
	p.state.Flows = map[string]*flowProgress{}
	if scenarios, ok := metadata["scenarios"].([]interface{}); ok {
		for _, raw := range scenarios {
			if f, ok := raw.(map[string]interface{}); ok {
				if name, ok := f["name"].(string); ok {
					p.state.Flows[name] = &flowProgress{}
				}
			}
		}
	}
	return p
}
func (p *progressCollector) add(containers []metrics.SampleContainer) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for _, container := range containers {
		for _, sample := range container.GetSamples() {
			name := sample.Metric.Name
			switch name {
			case "iterations":
				p.state.Iterations += uint64(sample.Value)
			case "dropped_iterations":
				p.state.Dropped += uint64(sample.Value)
			case "http_reqs":
				p.state.HTTPAttempts += uint64(sample.Value)
			case "http_req_failed":
				if sample.Value > 0 {
					p.state.HTTPErrors++
				}
			default:
				if strings.HasPrefix(name, "scenario_success_") {
					if f := p.state.Flows[strings.TrimPrefix(name, "scenario_success_")]; f != nil {
						f.Successes += uint64(sample.Value)
						f.windowSuccess += uint64(sample.Value)
					}
				}
				if strings.HasPrefix(name, "scenario_failure_") {
					if f := p.state.Flows[strings.TrimPrefix(name, "scenario_failure_")]; f != nil {
						f.Failures += uint64(sample.Value)
					}
				}
				if strings.HasPrefix(name, "payment_confirm_ms_") {
					if f := p.state.Flows[strings.TrimPrefix(name, "payment_confirm_ms_")]; f != nil && len(f.latencies) < 8192 {
						f.latencies = append(f.latencies, sample.Value)
					}
				}
			}
		}
	}
}
func (p *progressCollector) bytes(stats Stats) []byte {
	p.mu.Lock()
	defer p.mu.Unlock()
	now := time.Now()
	seconds := now.Sub(p.previous).Seconds()
	p.previous = now
	p.state.Elapsed = now.Sub(p.started).Seconds()
	p.state.Recorder = stats
	p.state.SuccessfulTPS = 0
	for _, f := range p.state.Flows {
		f.TPS = float64(f.windowSuccess) / seconds
		f.windowSuccess = 0
		p.state.SuccessfulTPS += f.TPS
		f.P90 = nil
		if len(f.latencies) > 0 {
			sort.Float64s(f.latencies)
			value := f.latencies[int(float64(len(f.latencies)-1)*0.9)]
			f.P90 = &value
			f.latencies = f.latencies[:0]
		}
	}
	raw, _ := json.Marshal(p.state)
	return raw
}
func (o *sqliteOutput) writeProgress() {
	if o.progressFile == "" {
		return
	}
	raw := o.progress.bytes(o.writer.Snapshot())
	if err := os.WriteFile(o.progressFile+".tmp", raw, 0600); err != nil {
		o.writer.fail(err)
		return
	}
	if err := os.Rename(o.progressFile+".tmp", o.progressFile); err != nil {
		o.writer.fail(err)
	}
}
