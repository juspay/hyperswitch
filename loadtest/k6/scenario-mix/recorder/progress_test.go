package recorder

import (
	"encoding/json"
	"go.k6.io/k6/metrics"
	"testing"
)

func TestProgressHTTPDisplayCountsPaymentConfirmOnly(t *testing.T) {
	p := newProgressCollector(map[string]interface{}{})
	r := metrics.NewRegistry()
	for _, operation := range []string{"pm_session_confirm", "payment_create", "payment_confirm"} {
		tags := r.RootTagSet().With("operation", operation)
		p.add([]metrics.SampleContainer{metrics.Samples{
			{TimeSeries: metrics.TimeSeries{Metric: r.MustNewMetric("http_reqs", metrics.Counter), Tags: tags}, Value: 10},
			{TimeSeries: metrics.TimeSeries{Metric: r.MustNewMetric("http_req_failed", metrics.Rate), Tags: tags}, Value: 1},
		}})
	}
	if p.state.HTTPAttempts != 30 || p.state.HTTPErrors != 3 || p.state.ConfirmHTTPAttempts != 10 || p.state.ConfirmHTTPErrors != 1 {
		t.Fatalf("wrong HTTP counters: %+v", p.state)
	}
}

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

func TestServerLatencyWindowPreservesZeroAndResets(t *testing.T) {
	p := newProgressCollector(map[string]interface{}{"scenarios": []interface{}{map[string]interface{}{"name": "guest"}}})
	r := metrics.NewRegistry()
	for _, name := range []string{"payment_confirm_server_latency_ms", "payment_confirm_server_ms_guest"} {
		metric := r.MustNewMetric(name, metrics.Trend)
		for _, value := range []float64{0, 10, 30} {
			p.add([]metrics.SampleContainer{metrics.Samples{{TimeSeries: metrics.TimeSeries{Metric: metric}, Value: value}}})
		}
	}
	var result liveProgress
	if err := json.Unmarshal(p.bytes(Stats{}), &result); err != nil {
		t.Fatal(err)
	}
	if result.ServerP99 == nil || *result.ServerP99 != 10 || result.Flows["guest"].ServerP99 == nil || *result.Flows["guest"].ServerP99 != 10 || len(result.ServerSamples) != 3 || result.ServerSamples[0] != 0 {
		t.Fatalf("wrong server latency: %+v", result)
	}
	if err := json.Unmarshal(p.bytes(Stats{}), &result); err != nil {
		t.Fatal(err)
	}
	if result.ServerP99 != nil || result.Flows["guest"].ServerP99 != nil {
		t.Fatal("missing samples reported as latency")
	}
}
