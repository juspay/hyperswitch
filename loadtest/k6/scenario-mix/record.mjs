// Node fixture response extraction. Go load capture uses the same SQLite schema.
// No request headers/card data or successful raw bodies are retained.
export function responseRecord(context, method, url, request, response, text, latencyMs, transportError = null, parsedBody = undefined) {
  let body = {};
  let parseError = null;
  if (parsedBody !== undefined) body = parsedBody;
  else if (text) {
    try { body = JSON.parse(text); } catch (_) { parseError = "invalid_json_response"; }
  }
  if (!body || typeof body !== "object") body = {};
  const operation = context.operation;
  const isCustomer = operation === "customer_create";
  const isPayment = /^(payment_create|payment_confirm|baseline_create|baseline_confirm|baseline_poll)$/.test(operation);
  const baseline = operation.startsWith("baseline_");
  const role = context.role || (baseline ? "baseline" : "measured");
  const pathId = /\/payments\/([^/?]+)(?:\/|\?|$)/.exec(url)?.[1];
  const customerId = (isCustomer ? body.id || body.customer_id : body.customer_id)
    || request?.customer_id || context.customer_id || null;
  const paymentId = isPayment ? body.payment_id || pathId || (operation === "payment_confirm" ? context.payment_id : null) || null : context.payment_id || null;
  const apiStatus = Number(response?.status || 0);
  const failed = apiStatus < 200 || apiStatus >= 300;
  const headers = response?.headers || {};
  let requestId = null;
  for (const key of Object.keys(headers)) {
    if (["x-request-id", "request-id", "request_id"].includes(key.toLowerCase())) {
      requestId = Array.isArray(headers[key]) ? headers[key][0] : headers[key];
      break;
    }
  }
  const message = transportError || (failed ? body.error?.message || (typeof body.error === "string" ? body.error : null) || body.message || response?.error || (parseError ? "non_json_error_response" : `HTTP ${apiStatus}`) : parseError);
  return {
    flow_id: context.flow_id, merchant_id: context.merchant_id || null,
    operation, role, observed_at: Date.now(), method, url,
    customer_id: customerId, merchant_reference_id: body.merchant_reference_id || request?.merchant_reference_id || null,
    payment_id: paymentId, amount: Number.isSafeInteger(body.amount) ? body.amount : Number.isSafeInteger(request?.amount) ? request.amount : null,
    status: isPayment && typeof body.status === "string" ? body.status : null,
    api_status_code: apiStatus, request_id: requestId || null, latency_ms: latencyMs,
    message: message == null ? null : String(message), error_body: failed ? text || null : null,
    entity_type: isCustomer ? "customer" : isPayment ? "payment" : null,
  };
}
