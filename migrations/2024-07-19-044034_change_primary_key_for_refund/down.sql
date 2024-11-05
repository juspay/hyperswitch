-- This file should undo anything in `up.sql`
ALTER TABLE refund DROP CONSTRAINT refund_pkey;

ALTER TABLE refund
ADD PRIMARY KEY (id);
