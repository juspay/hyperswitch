INSERT INTO customers
(run_id,customer_id,merchant_id,merchant_reference_id,api_status_code,request_id,message,observed_at,response_event_id)
VALUES (?,?,?,?,?,?,?,?,?)
ON CONFLICT(run_id,customer_id) DO UPDATE SET
 merchant_reference_id=COALESCE(excluded.merchant_reference_id,customers.merchant_reference_id),
 api_status_code=excluded.api_status_code,request_id=excluded.request_id,
 message=excluded.message,observed_at=excluded.observed_at,response_event_id=excluded.response_event_id
WHERE excluded.observed_at > customers.observed_at OR (excluded.observed_at = customers.observed_at AND excluded.response_event_id > customers.response_event_id)
