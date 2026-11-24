-- The scheduler's producer finds due jobs by status and schedule_time on every loop, and the
-- cleanup finds stuck jobs the same way; without this index each lookup scans the whole table,
-- which keeps every finished job.
CREATE INDEX CONCURRENTLY IF NOT EXISTS process_tracker_status_schedule_time_index
ON process_tracker (status, schedule_time);
