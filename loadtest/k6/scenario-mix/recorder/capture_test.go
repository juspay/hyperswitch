package recorder

import "testing"

func TestServerLatencyHeader(t *testing.T) {
	for text, want := range map[string]float64{"0": 0, " 12.5 ": 12.5} {
		got := parseServerLatency(text)
		if got == nil || *got != want {
			t.Fatalf("%q: %v", text, got)
		}
	}
	for _, text := range []string{"", "true", "NaN", "Infinity", "-1", "10ms", "1e2"} {
		if parseServerLatency(text) != nil {
			t.Fatalf("accepted invalid latency %q", text)
		}
	}
}
