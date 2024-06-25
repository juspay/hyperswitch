CREATE TYPE "GenericLinkType" as ENUM(
    'payment_method_collect',
    'payout_link'
);

CREATE TABLE generic_link (
  link_id VARCHAR (64) NOT NULL PRIMARY KEY,
  primary_reference VARCHAR (64) NOT NULL,
  merchant_id VARCHAR (64) NOT NULL,
  created_at timestamp NOT NULL DEFAULT NOW():: timestamp,
  last_modified_at timestamp NOT NULL DEFAULT NOW():: timestamp,
  expiry timestamp NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL '15 minutes'):: timestamp,
  link_data JSONB NOT NULL,
  link_status VARCHAR (256) NOT NULL,
  link_type "GenericLinkType" NOT NULL,
  url TEXT NOT NULL,
  return_url TEXT NULL
);