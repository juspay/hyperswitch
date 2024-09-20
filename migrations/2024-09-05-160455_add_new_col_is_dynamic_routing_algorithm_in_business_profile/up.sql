-- Your SQL goes here
ALTER TABLE
    business_profile
ADD
    COLUMN dynamic_routing_algorithm JSON DEFAULT NULL;