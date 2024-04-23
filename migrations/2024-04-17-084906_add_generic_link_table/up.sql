CREATE TYPE "GenericLinkType" as ENUM('payment_method_collect');

CREATE TABLE generic_link (
  link_id VARCHAR (64) NOT NULL PRIMARY KEY,
  primary_reference VARCHAR (64) NOT NULL,
  merchant_id VARCHAR (64) NOT NULL,
  created_at timestamp NOT NULL DEFAULT NOW():: timestamp,
  last_modified_at timestamp NOT NULL DEFAULT NOW():: timestamp,
  expiry timestamp NOT NULL DEFAULT NOW():: timestamp,
  link_data JSONB NOT NULL,
  link_status VARCHAR (32) NOT NULL,
  link_type "GenericLinkType" NOT NULL,
  url VARCHAR (256) NOT NULL,
  return_url VARCHAR(256) NULL
);