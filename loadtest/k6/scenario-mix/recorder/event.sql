INSERT INTO request_events
(run_id,attempt_id,flow_id,merchant_id,operation,role,observed_at,method,url,customer_id,
 merchant_reference_id,payment_id,amount,status,api_status_code,request_id,
 latency_ms,message,error_body,entity_type)
VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
