-- Your SQL goes here
-- Tables
CREATE TABLE IF NOT EXISTS gateway_status_map (
    connector VARCHAR(64) NOT NULL,
    flow VARCHAR(64) NOT NULL,
    sub_flow VARCHAR(64) NOT NULL,
    code VARCHAR(255) NOT NULL,
    message VARCHAR(1024),
    status VARCHAR(64) NOT NULL,
    router_error VARCHAR(64),
    decision VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    last_modified TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    step_up_possible BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (connector, flow, sub_flow, code, message)
);
