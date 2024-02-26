-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS authentication;
DROP TYPE "DecoupledAuthenticationType";
DROP TYPE "AuthenticationStatus";
DROP TYPE "AuthenticationLifecycleStatus";
