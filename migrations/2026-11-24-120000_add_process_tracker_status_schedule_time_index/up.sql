-- The scheduler finds the tasks it has to run by status and schedule time, in the producer and in
-- the consumer's cleanup, and without this index each of those queries scans the whole table.
CREATE INDEX CONCURRENTLY IF NOT EXISTS process_tracker_status_schedule_time_index
ON process_tracker (status, schedule_time);
