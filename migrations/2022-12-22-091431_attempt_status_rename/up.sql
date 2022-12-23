ALTER TYPE "AttemptStatus" RENAME VALUE 'juspay_declined' TO 'router_declined';
ALTER TYPE "AttemptStatus" RENAME VALUE 'pending_vbv' TO 'authentication_successful';
ALTER TYPE "AttemptStatus" RENAME VALUE 'vbv_successful' TO 'authentication_pending';
