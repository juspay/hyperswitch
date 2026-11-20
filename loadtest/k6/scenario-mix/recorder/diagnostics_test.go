package recorder

import (
	"strings"
	"testing"
)

func TestErrorDiagnosticsExcludeSensitiveFields(t *testing.T) {
	raw := `{"error":{"code":"PAYMENT_FAILED","message":"Invalid card 4242424242424242; api_key=private-key","metadata":{"password":"private-password"}},"client_secret":"private-client-secret"}`
	message := "Authorization: Bearer private-token"
	r := sanitizeDiagnostics(Record{URL: "http://router/confirm?client_secret=private-client-secret", Message: &message, ErrorBody: &raw})
	result := r.URL + *r.Message + *r.ErrorBody
	for _, secret := range []string{"4242424242424242", "private-key", "private-password", "private-client-secret", "private-token"} {
		if strings.Contains(result, secret) {
			t.Fatalf("retained sensitive diagnostic: %s", secret)
		}
	}
	if !strings.Contains(*r.ErrorBody, "PAYMENT_FAILED") {
		t.Fatal("lost diagnostic code")
	}
	if strings.Contains(diagnosticBody("<html>raw-secret</html>"), "raw-secret") {
		t.Fatal("retained raw non-JSON body")
	}
}
