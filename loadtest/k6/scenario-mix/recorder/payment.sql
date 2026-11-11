INSERT INTO payments
(run_id,payment_id,merchant_id,customer_id,amount,status,role,api_status_code,request_id,message,observed_at,response_event_id)
VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
ON CONFLICT(run_id,payment_id) DO UPDATE SET
 customer_id=COALESCE(excluded.customer_id,payments.customer_id),
 amount=COALESCE(excluded.amount,payments.amount),status=COALESCE(excluded.status,payments.status),
 api_status_code=excluded.api_status_code,request_id=excluded.request_id,
 message=excluded.message,observed_at=excluded.observed_at,response_event_id=excluded.response_event_id
WHERE excluded.observed_at > payments.observed_at OR (excluded.observed_at = payments.observed_at AND excluded.response_event_id > payments.response_event_id)
