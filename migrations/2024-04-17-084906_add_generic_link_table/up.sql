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
  expiry timestamp NOT NULL,
  link_data JSONB NOT NULL,
  link_status JSONB NOT NULL,
  link_type "GenericLinkType" NOT NULL,
  url TEXT NOT NULL,
  return_url TEXT NULL
);