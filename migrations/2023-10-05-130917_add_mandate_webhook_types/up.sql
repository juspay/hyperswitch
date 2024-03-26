-- Your SQL goes here
ALTER TYPE "EventClass" ADD VALUE 'mandates';

ALTER TYPE "EventObjectType" ADD VALUE 'mandate_details';

ALTER TYPE "EventType" ADD VALUE 'mandate_active';

ALTER TYPE "EventType" ADD VALUE 'mandate_revoked';