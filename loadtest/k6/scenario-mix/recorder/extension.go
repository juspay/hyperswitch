package recorder

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"sync"
	"time"

	"go.k6.io/k6/errext"
	"go.k6.io/k6/js/modules"
	"go.k6.io/k6/metrics"
	"go.k6.io/k6/output"
)

var active struct {
	sync.RWMutex
	writer *Writer
	last   *Stats
}

func init() {
	modules.Register("k6/x/sqlite-recorder", &module{})
	output.RegisterExtension("sqlite-recorder", newOutput)
}

type module struct{}
type instance struct{}

func (*module) NewModuleInstance(_ modules.VU) modules.Instance { return &instance{} }
func (*instance) Exports() modules.Exports {
	return modules.Exports{Default: map[string]interface{}{"record": record, "begin": begin, "finish": finish, "complete": complete, "stats": snapshot}}
}
func current() (*Writer, error) {
	active.RLock()
	defer active.RUnlock()
	if active.writer == nil {
		return nil, errors.New("SQLite recorder not started; use --out sqlite-recorder")
	}
	return active.writer, nil
}
func record(r Record) error {
	w, err := current()
	if err != nil {
		return err
	}
	return w.Enqueue(r)
}
func snapshot() (Stats, error) {
	active.RLock()
	defer active.RUnlock()
	if active.writer != nil {
		return active.writer.Snapshot(), nil
	}
	if active.last != nil {
		return *active.last, nil
	}
	return Stats{}, errors.New("SQLite recorder has no run")
}

type sqliteOutput struct {
	params      output.Params
	writer      *Writer
	stop        func(error)
	monitorStop chan struct{}
	monitorDone chan struct{}
}

func newOutput(params output.Params) (output.Output, error) {
	return &sqliteOutput{params: params}, nil
}
func (*sqliteOutput) Description() string                       { return "buffered SQLite response recorder" }
func (o *sqliteOutput) SetTestRunStopCallback(stop func(error)) { o.stop = stop }
func (o *sqliteOutput) Start() error {
	file := o.params.Environment["SQLITE_RECORDER_DB"]
	runID := o.params.Environment["SQLITE_RECORDER_RUN_ID"]
	if file == "" || runID == "" {
		return errors.New("SQLITE_RECORDER_DB and SQLITE_RECORDER_RUN_ID are required")
	}
	metadata := map[string]interface{}{}
	if raw := o.params.Environment["SQLITE_RECORDER_METADATA"]; raw != "" {
		if err := json.Unmarshal([]byte(raw), &metadata); err != nil {
			return err
		}
	}
	w, err := NewWriter(file, runID, DefaultSettings(), metadata)
	if err != nil {
		return err
	}
	w.SetStop(o.stop)
	o.writer = w
	active.Lock()
	active.writer = w
	active.Unlock()
	o.monitorStop = make(chan struct{})
	o.monitorDone = make(chan struct{})
	go func() {
		defer close(o.monitorDone)
		ticker := time.NewTicker(10 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-o.monitorStop:
				return
			case <-ticker.C:
				raw, _ := json.Marshal(w.Snapshot())
				fmt.Fprintf(o.params.StdErr, "SQLite recorder progress: %s\n", raw)
			}
		}
	}()
	return nil
}
func (*sqliteOutput) AddMetricSamples(_ []metrics.SampleContainer) {}
func (o *sqliteOutput) Stop() error                                { return o.StopWithTestError(nil) }
func (o *sqliteOutput) StopWithTestError(err error) error {
	if o.writer == nil {
		return nil
	}
	close(o.monitorStop)
	<-o.monitorDone
	result := o.writer.Close(recordingError(err))
	stats := o.writer.Snapshot()
	raw, _ := json.Marshal(stats)
	fmt.Fprintf(o.params.StdErr, "SQLite recorder: %s\n", raw)
	active.Lock()
	active.writer = nil
	active.last = &stats
	active.Unlock()
	return result
}

// A threshold failure after the test ends affects test success, not data completeness.
// Early threshold aborts and other interruptions still leave recording incomplete.
func recordingError(err error) error {
	var reason errext.HasAbortReason
	if errors.As(err, &reason) && reason.AbortReason() == errext.AbortedByThresholdsAfterTestEnd {
		return nil
	}
	return err
}

func begin(flowID string, merchantID *string, operation, method, url string, customerID, reference, paymentID *string, amount *int64, started int64) (uint64, error) {
	w, err := current()
	if err != nil {
		return 0, err
	}
	role := "measured"
	if strings.HasPrefix(operation, "baseline_") {
		role = "baseline"
	}
	var entity *string
	if operation == "customer_create" {
		entity = strPointer("customer")
	}
	switch operation {
	case "payment_create", "payment_confirm", "baseline_create", "baseline_confirm", "baseline_poll":
		entity = strPointer("payment")
		if i := strings.Index(url, "/payments/"); i >= 0 {
			id := strings.SplitN(strings.SplitN(url[i+len("/payments/"):], "/", 2)[0], "?", 2)[0]
			if id != "" {
				paymentID = &id
			}
		} else if operation == "payment_create" || operation == "baseline_create" {
			paymentID = nil
		}
	}
	return w.Begin(Record{FlowID: flowID, MerchantID: merchantID, Operation: operation, Role: role, Method: method, URL: url,
		CustomerID: customerID, MerchantReferenceID: reference, PaymentID: paymentID, Amount: amount, ObservedAt: time.Now().UnixMilli(), LatencyMS: float64(started), EntityType: entity})
}
func finish(id uint64, r Record) error {
	w, err := current()
	if err != nil {
		return err
	}
	return w.Finish(id, r)
}
