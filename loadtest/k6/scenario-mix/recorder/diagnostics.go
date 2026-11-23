package recorder

import (
	"encoding/json"
	"net/url"
	"regexp"
	"strings"
)

var diagnosticSecret = regexp.MustCompile(`(?i)(api[_ -]?key|client[_ -]?secret|authorization|password|access[_ -]?token|card[_ -]?(?:number|cvc)|cvc|cvv)(["']?\s*[=:]\s*)("[^"]*"|'[^']*'|[^\s;,]+)`)
var diagnosticBearer = regexp.MustCompile(`(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+`)
var diagnosticCard = regexp.MustCompile(`\b(?:\d[ -]?){13,19}\b`)
var diagnosticQuery = regexp.MustCompile(`(?i)key|secret|token|password|cvc|cvv|card`)

// diagnosticText redacts common credential, bearer-token, and card-number
// patterns, then bounds retained text to 512 runes. This is best-effort
// filtering for diagnostic strings, not a general sensitive-data detector.
func diagnosticText(value string) string {
	value = diagnosticBearer.ReplaceAllString(value, "Bearer [redacted]")
	value = diagnosticSecret.ReplaceAllString(value, "${1}${2}[redacted]")
	value = diagnosticCard.ReplaceAllString(value, "[redacted]")
	text := []rune(value)
	if len(text) > 512 {
		return string(text[:512])
	}
	return value
}

// diagnosticProjection retains only error, code, type, and message fields
// from nested JSON objects, sanitizing string leaves. Unsupported values and
// objects beyond depth three become nil rather than retaining arbitrary bodies.
func diagnosticProjection(value interface{}, depth int) interface{} {
	if text, ok := value.(string); ok {
		return diagnosticText(text)
	}
	object, ok := value.(map[string]interface{})
	if !ok || depth > 3 {
		return nil
	}
	result := map[string]interface{}{}
	for _, key := range []string{"error", "code", "type", "message"} {
		if field, ok := object[key]; ok {
			result[key] = diagnosticProjection(field, depth+1)
		}
	}
	return result
}

// diagnosticBody produces a bounded JSON error projection for persistence.
// It omits oversized or non-JSON input and replaces projections exceeding
// the output limit with a fixed diagnostic message.
func diagnosticBody(text string) string {
	if len(text) > 65536 {
		return "[oversized error response omitted]"
	}
	var value interface{}
	if json.Unmarshal([]byte(text), &value) != nil {
		return "[non-JSON error response omitted]"
	}
	data, err := json.Marshal(diagnosticProjection(value, 0))
	if err != nil || len(data) > 4096 {
		return `{"message":"Error diagnostic exceeded limit"}`
	}
	return string(data)
}

// sanitizeDiagnostics returns a record with filtered error text and body,
// URL user information removed, and sensitive-looking query values redacted.
// Malformed URLs fall back to bounded text filtering; other record fields
// are preserved, so callers must still avoid capturing request secrets.
func sanitizeDiagnostics(r Record) Record {
	if r.Message != nil {
		text := diagnosticText(*r.Message)
		r.Message = &text
	}
	if r.ErrorBody != nil {
		text := diagnosticBody(*r.ErrorBody)
		r.ErrorBody = &text
	}
	if u, err := url.Parse(r.URL); err == nil {
		u.User = nil
		query := u.Query()
		changed := false
		for key := range query {
			if diagnosticQuery.MatchString(key) {
				query.Set(key, "[redacted]")
				changed = true
			}
		}
		if changed {
			u.RawQuery = query.Encode()
		}
		r.URL = u.String()
	} else {
		r.URL = strings.TrimSpace(diagnosticText(r.URL))
	}
	return r
}
