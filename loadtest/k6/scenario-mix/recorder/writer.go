package recorder

import (
	"database/sql"
	_ "embed"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sync"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

//go:embed schema.sql
var schemaSQL string

//go:embed event.sql
var eventSQL string

//go:embed customer.sql
var customerSQL string

//go:embed payment.sql
var paymentSQL string

var errQueueFull = errors.New("SQLite recording queue exhausted; run is incomplete")

// Record contains only response projections, never request headers or card data.
type Record struct {
	AttemptID           uint64  `json:"attempt_id" js:"attempt_id"`
	FlowID              string  `json:"flow_id" js:"flow_id"`
	MerchantID          *string `json:"merchant_id" js:"merchant_id"`
	Operation           string  `json:"operation" js:"operation"`
	Role                string  `json:"role" js:"role"`
	ObservedAt          int64   `json:"observed_at" js:"observed_at"`
	Method              string  `json:"method" js:"method"`
	URL                 string  `json:"url" js:"url"`
	CustomerID          *string `json:"customer_id" js:"customer_id"`
	MerchantReferenceID *string `json:"merchant_reference_id" js:"merchant_reference_id"`
	PaymentID           *string `json:"payment_id" js:"payment_id"`
	Amount              *int64  `json:"amount" js:"amount"`
	Status              *string `json:"status" js:"status"`
	APIStatusCode       int     `json:"api_status_code" js:"api_status_code"`
	RequestID           *string `json:"request_id" js:"request_id"`
	LatencyMS           float64 `json:"latency_ms" js:"latency_ms"`
	Message             *string `json:"message" js:"message"`
	ErrorBody           *string `json:"error_body" js:"error_body"`
	EntityType          *string `json:"entity_type" js:"entity_type"`
	size                int64
}

func (r *Record) bytes() int64 {
	n := int64(384 + len(r.FlowID) + len(r.Operation) + len(r.Role) + len(r.Method) + len(r.URL))
	for _, s := range []*string{r.MerchantID, r.CustomerID, r.MerchantReferenceID, r.PaymentID, r.Status, r.RequestID, r.Message, r.ErrorBody, r.EntityType} {
		if s != nil {
			n += int64(len(*s))
		}
	}
	return n
}

type Settings struct {
	BatchSize, MaxEvents int
	MaxBytes             int64
	FlushInterval        time.Duration
}

func DefaultSettings() Settings { return Settings{1000, 100000, 64 << 20, 50 * time.Millisecond} }

type Stats struct {
	Attempts      uint64  `json:"attempts" js:"attempts"`
	PendingEvents int     `json:"pending_events" js:"pending_events"`
	PendingBytes  int64   `json:"pending_bytes" js:"pending_bytes"`
	Enqueued      uint64  `json:"enqueued" js:"enqueued"`
	Persisted     uint64  `json:"persisted" js:"persisted"`
	QueueEvents   int     `json:"queue_events" js:"queue_events"`
	QueueBytes    int64   `json:"queue_bytes" js:"queue_bytes"`
	PeakEvents    int     `json:"peak_events" js:"peak_events"`
	PeakBytes     int64   `json:"peak_bytes" js:"peak_bytes"`
	MaxCommitMS   float64 `json:"max_commit_ms" js:"max_commit_ms"`
	Error         string  `json:"error,omitempty" js:"error"`
}

type Writer struct {
	db       *sql.DB
	runID    string
	settings Settings
	queue    chan *Record
	done     chan struct{}
	mu       sync.Mutex
	stats    Stats
	err      error
	closed   bool
	closing  bool
	pending  map[uint64]Record
	stop     func(error)
	metadata map[string]interface{}
}

func NewWriter(file, runID string, settings Settings, metadata map[string]interface{}) (*Writer, error) {
	if settings.BatchSize < 1 || settings.MaxEvents < 1 || settings.MaxBytes < 1 || settings.FlushInterval <= 0 {
		return nil, errors.New("invalid recorder settings")
	}
	// A database belongs to one generator/run. Never overwrite an earlier run.
	f, err := os.OpenFile(file, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if err != nil {
		return nil, err
	}
	if err = f.Close(); err != nil {
		return nil, err
	}
	db, err := sql.Open("sqlite3", file)
	if err != nil {
		return nil, err
	}
	db.SetMaxOpenConns(1)
	if _, err = db.Exec(schemaSQL); err != nil {
		db.Close()
		return nil, err
	}
	if metadata == nil {
		metadata = map[string]interface{}{}
	}
	raw, _ := json.Marshal(metadata)
	if _, err = db.Exec("INSERT INTO runs(run_id,started_at,metadata) VALUES(?,?,?)", runID, time.Now().UnixMilli(), string(raw)); err != nil {
		db.Close()
		return nil, err
	}
	w := &Writer{db: db, runID: runID, settings: settings, queue: make(chan *Record, settings.MaxEvents), done: make(chan struct{}), metadata: metadata, pending: make(map[uint64]Record)}
	go w.loop()
	return w, nil
}

func (w *Writer) fail(err error) {
	w.mu.Lock()
	first := w.err == nil
	if first {
		w.err = err
		w.stats.Error = err.Error()
	}
	stop := w.stop
	w.mu.Unlock()
	if first && stop != nil {
		stop(err)
	}
}

func (w *Writer) SetStop(stop func(error)) { w.mu.Lock(); w.stop = stop; w.mu.Unlock() }

func (w *Writer) Enqueue(r Record) error {
	r.size = r.bytes()
	w.mu.Lock()
	if w.err != nil {
		err := w.err
		w.mu.Unlock()
		return err
	}
	if w.closed {
		w.mu.Unlock()
		return errors.New("recorder already closed")
	}
	if w.stats.QueueEvents+w.stats.PendingEvents >= w.settings.MaxEvents || w.stats.QueueBytes+w.stats.PendingBytes+r.size > w.settings.MaxBytes {
		w.mu.Unlock()
		err := errQueueFull
		w.fail(err)
		return err
	}
	w.stats.Enqueued++
	w.stats.QueueEvents++
	w.stats.QueueBytes += r.size
	if w.stats.QueueEvents > w.stats.PeakEvents {
		w.stats.PeakEvents = w.stats.QueueEvents
	}
	if w.stats.QueueBytes > w.stats.PeakBytes {
		w.stats.PeakBytes = w.stats.QueueBytes
	}
	// Count includes in-flight batches, so the buffered send is guaranteed to fit.
	w.queue <- &r
	w.mu.Unlock()
	return nil
}
func (w *Writer) Snapshot() Stats { w.mu.Lock(); defer w.mu.Unlock(); return w.stats }

func (w *Writer) loop() {
	defer close(w.done)
	ticker := time.NewTicker(w.settings.FlushInterval)
	defer ticker.Stop()
	batch := make([]*Record, 0, w.settings.BatchSize)
	flush := func() {
		if len(batch) == 0 {
			return
		}
		w.mu.Lock()
		fatal := w.err
		w.mu.Unlock()
		// Queue exhaustion still drains all accepted events. Database errors cannot.
		if fatal != nil && !errors.Is(fatal, errQueueFull) {
			return
		}
		if err := w.commit(batch); err != nil {
			w.fail(err)
			return
		}
		batch = batch[:0]
	}
	for {
		select {
		case r, ok := <-w.queue:
			if !ok {
				flush()
				return
			}
			batch = append(batch, r)
			if len(batch) >= w.settings.BatchSize {
				flush()
				if len(batch) != 0 {
					return
				}
			}
		case <-ticker.C:
			flush()
			if len(batch) != 0 {
				return
			}
		}
	}
}

func (w *Writer) commit(batch []*Record) error {
	start := time.Now()
	tx, err := w.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback() // no-op after commit
	insert, err := tx.Prepare(eventSQL)
	if err != nil {
		return err
	}
	defer insert.Close()
	customer, err := tx.Prepare(customerSQL)
	if err != nil {
		return err
	}
	defer customer.Close()
	payment, err := tx.Prepare(paymentSQL)
	if err != nil {
		return err
	}
	defer payment.Close()
	var bytes int64
	for _, r := range batch {
		result, err := insert.Exec(w.runID, r.AttemptID, r.FlowID, r.MerchantID, r.Operation, r.Role, r.ObservedAt, r.Method, r.URL, r.CustomerID, r.MerchantReferenceID, r.PaymentID, r.Amount, r.Status, r.APIStatusCode, r.RequestID, r.LatencyMS, r.Message, r.ErrorBody, r.EntityType)
		if err != nil {
			return err
		}
		id, err := result.LastInsertId()
		if err != nil {
			return err
		}
		if r.EntityType != nil && *r.EntityType == "customer" && r.CustomerID != nil {
			if _, err = customer.Exec(w.runID, r.CustomerID, r.MerchantID, r.MerchantReferenceID, r.APIStatusCode, r.RequestID, r.Message, r.ObservedAt, id); err != nil {
				return err
			}
		}
		if r.EntityType != nil && *r.EntityType == "payment" && r.PaymentID != nil {
			if _, err = payment.Exec(w.runID, r.PaymentID, r.MerchantID, r.CustomerID, r.Amount, r.Status, r.Role, r.APIStatusCode, r.RequestID, r.Message, r.ObservedAt, id); err != nil {
				return err
			}
		}
		bytes += r.size
	}
	w.mu.Lock()
	persisted := w.stats.Persisted + uint64(len(batch))
	enqueued := w.stats.Enqueued
	w.mu.Unlock()
	if _, err = tx.Exec("UPDATE runs SET enqueued=?,persisted=? WHERE run_id=?", enqueued, persisted, w.runID); err != nil {
		return err
	}
	if err = tx.Commit(); err != nil {
		return err
	}
	w.mu.Lock()
	w.stats.Persisted = persisted
	w.stats.QueueEvents -= len(batch)
	w.stats.QueueBytes -= bytes
	ms := float64(time.Since(start).Microseconds()) / 1000
	if ms > w.stats.MaxCommitMS {
		w.stats.MaxCommitMS = ms
	}
	w.mu.Unlock()
	return nil
}

// Begin reserves a bounded slot before an HTTP attempt. Interrupted requests become
// explicit transport-failure events at shutdown rather than disappearing.
func (w *Writer) Begin(r Record) (uint64, error) {
	r.size = r.bytes()
	w.mu.Lock()
	if w.err != nil {
		err := w.err
		w.mu.Unlock()
		return 0, err
	}
	if w.closed || w.closing {
		w.mu.Unlock()
		return 0, errors.New("recorder closing")
	}
	if w.stats.QueueEvents+w.stats.PendingEvents >= w.settings.MaxEvents || w.stats.QueueBytes+w.stats.PendingBytes+r.size > w.settings.MaxBytes {
		w.mu.Unlock()
		w.fail(errQueueFull)
		return 0, errQueueFull
	}
	w.stats.Attempts++
	r.AttemptID = w.stats.Attempts
	w.pending[r.AttemptID] = r
	w.stats.PendingEvents++
	w.stats.PendingBytes += r.size
	w.mu.Unlock()
	return r.AttemptID, nil
}
func (w *Writer) Finish(id uint64, r Record) error {
	w.mu.Lock()
	pending, ok := w.pending[id]
	if !ok {
		w.mu.Unlock()
		return errors.New("unknown/already recorded HTTP attempt")
	}
	delete(w.pending, id)
	w.stats.PendingEvents--
	w.stats.PendingBytes -= pending.size
	w.mu.Unlock()
	r.AttemptID = id
	return w.Enqueue(r)
}

func (w *Writer) Close(testErr error) error {
	w.mu.Lock()
	w.closing = true
	pending := w.pending
	w.pending = make(map[uint64]Record)
	w.stats.PendingEvents = 0
	w.stats.PendingBytes = 0
	w.mu.Unlock()
	if len(pending) > 0 && testErr == nil {
		testErr = errors.New("unfinished HTTP attempts at shutdown")
	}
	for _, r := range pending {
		r.APIStatusCode = 0
		r.ObservedAt = time.Now().UnixMilli()
		r.Message = strPointer("request interrupted before response was recorded")
		r.LatencyMS = float64(r.ObservedAt) - r.LatencyMS
		_ = w.Enqueue(r) // recording failures already mark the run incomplete
	}
	w.mu.Lock()
	if !w.closed {
		w.closed = true
		close(w.queue)
	}
	w.mu.Unlock()
	<-w.done
	stats := w.Snapshot()
	state := "complete"
	w.mu.Lock()
	err := w.err
	w.mu.Unlock()
	if err == nil {
		err = testErr
	}
	if err != nil || stats.Enqueued != stats.Persisted || (stats.Attempts > 0 && stats.Attempts != stats.Enqueued) {
		state = "incomplete"
	}
	w.metadata["recorder"] = stats
	raw, _ := json.Marshal(w.metadata)
	var message interface{}
	if err != nil {
		message = err.Error()
	}
	_, updateErr := w.db.Exec("UPDATE runs SET recording_state=?,ended_at=?,enqueued=?,persisted=?,error=?,metadata=? WHERE run_id=?", state, time.Now().UnixMilli(), stats.Enqueued, stats.Persisted, message, string(raw), w.runID)
	closeErr := w.db.Close()
	if err != nil {
		return fmt.Errorf("recording incomplete: %w", err)
	}
	if updateErr != nil {
		return updateErr
	}
	return closeErr
}

func strPointer(s string) *string { return &s }
