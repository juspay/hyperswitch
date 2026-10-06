package recorder

import (
	"fmt"
	"math"
	"strings"
	"time"

	"github.com/grafana/sobek"
	k6http "go.k6.io/k6/js/modules/k6/http"
)

// Reading the native Response avoids repeatedly exporting its headers/timings and
// 20 record fields across the JS bridge. JSON() reuses k6's own JSON cache; Export
// on its Go map returns the underlying map, rather than copying its properties.
type Captured struct {
	CustomerID *string `js:"customer_id"`
	PaymentID  *string `js:"payment_id"`
	EntityType *string `js:"entity_type"`
	Role       string  `js:"role"`
	RequestID  *string `js:"request_id"`
}

func parsedBody(response *k6http.Response) (body map[string]interface{}, malformed bool) {
	defer func() {
		if panicValue := recover(); panicValue != nil {
			switch panicValue.(type) {
			case *sobek.Exception, *sobek.Object:
				body = nil
				malformed = true
			default:
				panic(panicValue)
			}
		}
	}()
	body, _ = response.JSON().Export().(map[string]interface{})
	return body, false
}
func stringField(body map[string]interface{}, key string) *string {
	if value, ok := body[key].(string); ok && value != "" {
		return &value
	}
	return nil
}
func complete(id uint64, response *k6http.Response, transportError string) (Captured, error) {
	w, err := current()
	if err != nil {
		return Captured{}, err
	}
	w.mu.Lock()
	r, ok := w.pending[id]
	w.mu.Unlock()
	if !ok {
		return Captured{}, fmt.Errorf("unknown HTTP attempt %d", id)
	}
	started := r.LatencyMS
	r.ObservedAt = time.Now().UnixMilli()
	r.LatencyMS = float64(r.ObservedAt) - started
	var body map[string]interface{}
	malformed := false
	if response != nil {
		r.APIStatusCode = response.Status
		r.LatencyMS = response.Timings.Duration
		for key, value := range response.Headers {
			switch strings.ToLower(key) {
			case "x-request-id", "request-id", "request_id":
				r.RequestID = strPointer(value)
			}
		}
		failed := r.APIStatusCode < 200 || r.APIStatusCode >= 300
		text, _ := response.Body.(string)
		if raw, ok := response.Body.([]byte); ok {
			text = string(raw)
		}
		if failed && text != "" {
			r.ErrorBody = strPointer(text)
		}
		// Successful non-entity steps need only metadata. Their JSON is parsed by the
		// scenario if needed; entity/error responses share that same cached parse.
		if text != "" && (r.EntityType != nil || failed) {
			body, malformed = parsedBody(response)
		}
		if failed {
			message := fmt.Sprintf("HTTP %d", r.APIStatusCode)
			if malformed {
				message = "non_json_error_response"
			}
			if response.Error != "" {
				message = response.Error
			}
			if value := stringField(body, "message"); value != nil {
				message = *value
			}
			if value := stringField(body, "error"); value != nil {
				message = *value
			}
			if nested, ok := body["error"].(map[string]interface{}); ok {
				if value := stringField(nested, "message"); value != nil {
					message = *value
				}
			}
			r.Message = strPointer(message)
		} else if malformed {
			r.Message = strPointer("invalid_json_response")
		}
	}
	if response == nil && transportError == "" {
		r.Message = strPointer("no_http_response")
	}
	if transportError != "" {
		r.Message = strPointer(transportError)
	}
	if r.EntityType != nil {
		if *r.EntityType == "customer" {
			if value := stringField(body, "id"); value != nil {
				r.CustomerID = value
			} else if value := stringField(body, "customer_id"); value != nil {
				r.CustomerID = value
			}
			if value := stringField(body, "merchant_reference_id"); value != nil {
				r.MerchantReferenceID = value
			}
		} else if *r.EntityType == "payment" {
			if value := stringField(body, "payment_id"); value != nil {
				r.PaymentID = value
			}
			if value := stringField(body, "customer_id"); value != nil {
				r.CustomerID = value
			}
			if value := stringField(body, "status"); value != nil {
				r.Status = value
			}
			if value, ok := body["amount"].(float64); ok && math.Abs(value) <= 9007199254740991 && math.Trunc(value) == value {
				amount := int64(value)
				r.Amount = &amount
			}
		}
	}
	if err = w.Finish(id, r); err != nil {
		return Captured{}, err
	}
	return Captured{r.CustomerID, r.PaymentID, r.EntityType, r.Role, r.RequestID}, nil
}
