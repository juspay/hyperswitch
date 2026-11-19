package recorder

import (
	"database/sql"
	"errors"
	"fmt"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"go.k6.io/k6/errext"
)

func TestRecordingCompletenessIndependentOfFinalThresholds(t *testing.T) {
	for _, reason := range []errext.AbortReason{errext.AbortedByThresholdsAfterTestEnd, errext.AbortedByThreshold, errext.AbortedByUser, errext.AbortedByScriptError} {
		err := errext.WithAbortReasonIfNone(errors.New("test failed"), reason)
		if (recordingError(err) == nil) != (reason == errext.AbortedByThresholdsAfterTestEnd) {
			t.Fatalf("unexpected recording classification for %v", reason)
		}
	}
}

func str(s string) *string { return &s }
func paymentRecord(id, status string) Record {
	return Record{FlowID: "flow", Operation: "payment_confirm", Role: "measured", ObservedAt: time.Now().UnixMilli(), Method: "POST", URL: "http://test/payments/" + id + "/confirm", PaymentID: str(id), Status: str(status), APIStatusCode: 200, EntityType: str("payment")}
}
func TestConcurrentDrain(t *testing.T) {
	file := filepath.Join(t.TempDir(), "run.sqlite")
	w, err := NewWriter(file, "run", DefaultSettings(), nil)
	if err != nil {
		t.Fatal(err)
	}
	var wg sync.WaitGroup
	for vu := 0; vu < 8; vu++ {
		wg.Add(1)
		go func(vu int) {
			defer wg.Done()
			for i := 0; i < 200; i++ {
				id := fmt.Sprintf("pay-%d-%d", vu, i)
				attempt, err := w.Begin(paymentRecord(id, "processing"))
				if err != nil {
					t.Error(err)
					return
				}
				if err := w.Finish(attempt, paymentRecord(id, "processing")); err != nil {
					t.Error(err)
					return
				}
				attempt, err = w.Begin(paymentRecord(id, "succeeded"))
				if err != nil {
					t.Error(err)
					return
				}
				if err := w.Finish(attempt, paymentRecord(id, "succeeded")); err != nil {
					t.Error(err)
					return
				}
			}
		}(vu)
	}
	wg.Wait()
	if err := w.Close(nil); err != nil {
		t.Fatal(err)
	}
	stats := w.Snapshot()
	if stats.Enqueued != 3200 || stats.Persisted != 3200 || stats.QueueEvents != 0 {
		t.Fatalf("%+v", stats)
	}
	db, err := sql.Open("sqlite3", file)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	var count int
	if err = db.QueryRow("SELECT COUNT(*) FROM payments WHERE status='succeeded'").Scan(&count); err != nil || count != 1600 {
		t.Fatalf("count=%d err=%v", count, err)
	}
}
func TestByteQueueExhaustionAbortsAndMarksIncomplete(t *testing.T) {
	settings := DefaultSettings()
	settings.MaxBytes = 1
	file := filepath.Join(t.TempDir(), "run.sqlite")
	w, err := NewWriter(file, "run", settings, nil)
	if err != nil {
		t.Fatal(err)
	}
	called := false
	w.SetStop(func(error) { called = true })
	if err = w.Enqueue(paymentRecord("pay", "succeeded")); err == nil {
		t.Fatal("expected queue error")
	}
	if !called {
		t.Fatal("test-wide abort not called")
	}
	if err = w.Close(nil); err == nil {
		t.Fatal("expected incomplete close")
	}
	db, _ := sql.Open("sqlite3", file)
	defer db.Close()
	var state string
	if err = db.QueryRow("SELECT recording_state FROM runs").Scan(&state); err != nil || state != "incomplete" {
		t.Fatalf("state=%s err=%v", state, err)
	}
}
func TestDatabaseFailureAndGracefulAbort(t *testing.T) {
	t.Run("write failure", func(t *testing.T) {
		file := filepath.Join(t.TempDir(), "run.sqlite")
		w, err := NewWriter(file, "run", DefaultSettings(), nil)
		if err != nil {
			t.Fatal(err)
		}
		if _, err = w.db.Exec("DROP TABLE payments"); err != nil {
			t.Fatal(err)
		}
		if err = w.Enqueue(paymentRecord("pay", "succeeded")); err != nil {
			t.Fatal(err)
		}
		if err = w.Close(nil); err == nil {
			t.Fatal("expected write failure")
		}
		if w.Snapshot().Persisted != 0 {
			t.Fatal("failed transaction partially committed")
		}
	})
	t.Run("disk full", func(t *testing.T) {
		file := filepath.Join(t.TempDir(), "run.sqlite")
		w, err := NewWriter(file, "run", DefaultSettings(), nil)
		if err != nil {
			t.Fatal(err)
		}
		var pages int
		if err = w.db.QueryRow("PRAGMA page_count").Scan(&pages); err != nil {
			t.Fatal(err)
		}
		if _, err = w.db.Exec(fmt.Sprintf("PRAGMA max_page_count=%d", pages)); err != nil {
			t.Fatal(err)
		}
		r := paymentRecord("pay", "failed")
		r.ErrorBody = str(string(make([]byte, 1<<20)))
		if err = w.Enqueue(r); err != nil {
			t.Fatal(err)
		}
		if err = w.Close(nil); err == nil {
			t.Fatal("expected SQLITE_FULL")
		}
	})
	t.Run("graceful interrupt drains", func(t *testing.T) {
		file := filepath.Join(t.TempDir(), "run.sqlite")
		w, err := NewWriter(file, "run", DefaultSettings(), nil)
		if err != nil {
			t.Fatal(err)
		}
		for i := 0; i < 10; i++ {
			if err = w.Enqueue(paymentRecord(fmt.Sprint(i), "succeeded")); err != nil {
				t.Fatal(err)
			}
		}
		if err = w.Close(errors.New("interrupted")); err == nil {
			t.Fatal("expected incomplete abort")
		}
		if w.Snapshot().Persisted != 10 {
			t.Fatal("graceful abort lost accepted events")
		}
	})
}
func TestRefusesExistingDatabase(t *testing.T) {
	file := filepath.Join(t.TempDir(), "run.sqlite")
	w, err := NewWriter(file, "run", DefaultSettings(), nil)
	if err != nil {
		t.Fatal(err)
	}
	if err = w.Close(nil); err != nil {
		t.Fatal(err)
	}
	if _, err = NewWriter(file, "new", DefaultSettings(), nil); err == nil {
		t.Fatal("overwrote previous run")
	}
}
func BenchmarkWriter(b *testing.B) {
	settings := DefaultSettings()
	settings.MaxEvents = 1000000
	settings.MaxBytes = 256 << 20
	w, err := NewWriter(filepath.Join(b.TempDir(), "run.sqlite"), "bench", settings, nil)
	if err != nil {
		b.Fatal(err)
	}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		r := paymentRecord(fmt.Sprint(i), "succeeded")
		r.FlowID = "00000000-0000-0000-0000-000000000000:nomod_saved_card:100:123456"
		r.MerchantID = str("loadtest_mix_0001")
		r.CustomerID = str("cus_00000000000000000000000000000000")
		r.MerchantReferenceID = str("customer_mix_nomod_saved_card_123456_100_1787303017640")
		r.RequestID = str("00000000-0000-0000-0000-000000000000")
		r.LatencyMS = 3.5
		amount := int64(1000)
		r.Amount = &amount
		if err = w.Enqueue(r); err != nil {
			b.Fatal(err)
		}
	}
	if err = w.Close(nil); err != nil {
		b.Fatal(err)
	}
	b.StopTimer()
	b.ReportMetric(float64(b.N)/b.Elapsed().Seconds(), "events/s")
}

func TestInterruptedAttemptIsRecorded(t *testing.T) {
	file := filepath.Join(t.TempDir(), "run.sqlite")
	w, err := NewWriter(file, "run", DefaultSettings(), nil)
	if err != nil {
		t.Fatal(err)
	}
	r := paymentRecord("pay", "processing")
	r.Status = nil
	r.LatencyMS = float64(time.Now().UnixMilli())
	if _, err = w.Begin(r); err != nil {
		t.Fatal(err)
	}
	if err = w.Close(errors.New("SIGTERM")); err == nil {
		t.Fatal("expected incomplete run")
	}
	db, _ := sql.Open("sqlite3", file)
	defer db.Close()
	var status int
	var message string
	if err = db.QueryRow("SELECT api_status_code,message FROM request_events").Scan(&status, &message); err != nil {
		t.Fatal(err)
	}
	if status != 0 || message != "request interrupted before response was recorded" {
		t.Fatalf("status=%d message=%s", status, message)
	}
	if w.Snapshot().Enqueued != 1 || w.Snapshot().Persisted != 1 {
		t.Fatal("interrupted attempt lost")
	}
}
