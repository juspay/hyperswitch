package recorder

import (
	"encoding/json"
	"go.k6.io/k6/metrics"
	"testing"
)

func TestProgressCountsSuccessesWithoutDoubleCountingPhases(t *testing.T) {
	p := newProgressCollector(map[string]interface{}{"scenarios": []interface{}{map[string]interface{}{"name": "guest"}}})
	r := metrics.NewRegistry()
	samples := metrics.Samples{}
	for name, value := range map[string]float64{"scenario_success_guest": 10, "scenario_success_guest_p1": 10, "scenario_failure_guest": 2, "iterations": 12, "http_reqs": 40, "http_req_failed": 1, "payment_confirm_ms_guest": 123} {
		samples = append(samples, metrics.Sample{TimeSeries: metrics.TimeSeries{Metric: r.MustNewMetric(name, metrics.Counter)}, Value: value})
	}
	p.add([]metrics.SampleContainer{samples})
	var result liveProgress
	if err := json.Unmarshal(p.bytes(Stats{Enqueued: 40, Persisted: 39}), &result); err != nil {
		t.Fatal(err)
	}
	if result.Flows["guest"].Successes != 10 || result.Flows["guest"].Failures != 2 || result.Iterations != 12 {
		t.Fatalf("wrong counters: %+v", result)
	}
	if result.Flows["guest"].P90 == nil || *result.Flows["guest"].P90 != 123 {
		t.Fatal("latency missing")
	}
	if result.Recorder.Persisted != 39 {
		t.Fatal("recorder snapshot missing")
	}
}

func TestProgressLatencyWindowIsBoundedAndResets(t *testing.T) {
	p := newProgressCollector(map[string]interface{}{"scenarios": []interface{}{map[string]interface{}{"name": "guest"}}})
	r := metrics.NewRegistry()
	metric := r.MustNewMetric("payment_confirm_ms_guest", metrics.Trend)
	samples := make(metrics.Samples, 20000)
	for i := range samples {
		samples[i] = metrics.Sample{TimeSeries: metrics.TimeSeries{Metric: metric}, Value: float64(i)}
	}
	p.add([]metrics.SampleContainer{samples})
	if len(p.state.Flows["guest"].latencies) != 8192 {
		t.Fatal("unbounded latency window")
	}
	p.bytes(Stats{})
	if len(p.state.Flows["guest"].latencies) != 0 {
		t.Fatal("window did not reset")
	}
}
