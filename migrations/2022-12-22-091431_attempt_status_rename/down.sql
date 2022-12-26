ALTER TYPE "AttemptStatus" RENAME VALUE 'router_declined' TO 'juspay_declined';
ALTER TYPE "AttemptStatus" RENAME VALUE 'authentication_successful' TO 'pending_vbv';
ALTER TYPE "AttemptStatus" RENAME VALUE 'authentication_pending' TO 'vbv_successful';
