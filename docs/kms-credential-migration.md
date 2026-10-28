# KMS credential migration

The new environment fields contain KMS ciphertext. The application decrypts them at startup and stores the resulting credential in masked runtime configuration. Decryption failure or an empty decrypted credential fails startup; the application does not fall back to a legacy plaintext password or use ciphertext as a credential.

New optional password fields require a KMS secrets-management backend when populated. The `no_encryption` backend is rejected for these fields. When the primary backend is HashiCorp Vault, the independent token KMS client also decrypts these new fields. Existing fields outside this migration keep their existing behavior.

## Environment names

| Existing environment variable | New environment variable | New-version behavior |
|---|---|---|
| `ROUTER__INTERNAL_MERCHANT_ID_PROFILE_ID_AUTH__INTERNAL_API_KEY` | `ROUTER__INTERNAL_MERCHANT_ID_PROFILE_ID_AUTH__PRIMARY_INTERNAL_API_KEY` | Decrypts the new key; uses the old key only under the explicit rotation controls below. |
| `ROUTER__REDIS__PASSWORD` | `ROUTER__REDIS__PRIMARY_PASSWORD` | Ignores the old configuration field; decrypts the new field when present. |
| `DRAINER__REDIS__PASSWORD` | `DRAINER__REDIS__PRIMARY_PASSWORD` | The drainer follows the same Redis migration. |
| `ROUTER__ANALYTICS__CLICKHOUSE__PASSWORD` | `ROUTER__ANALYTICS__CLICKHOUSE__PRIMARY_PASSWORD` | Ignores the old field; decrypts the new field in ClickHouse and combined analytics modes. |
| `ROUTER__OPENSEARCH__AUTH__PASSWORD` | `ROUTER__OPENSEARCH__AUTH__PRIMARY_PASSWORD` | Ignores the old field; decrypts the new field when OpenSearch is enabled with Basic authentication. AWS authentication does not use it. |
| `ROUTER__EMAIL__SMTP__PASSWORD` | `ROUTER__EMAIL__SMTP__PRIMARY_PASSWORD` | Ignores the old field; decrypts the new field before SMTP client creation. |
| `ROUTER__EMAIL__SMTP__SOCKS5__PASSWORD` | `ROUTER__EMAIL__SMTP__SOCKS5__PRIMARY_PASSWORD` | Ignores the old field; decrypts the new field before proxy authentication. |
| `ROUTER__SECRETS_MANAGEMENT__HC_VAULT__TOKEN` | `ROUTER__SECRETS_MANAGEMENT__HC_VAULT__ENCRYPTED_TOKEN` | Ignores the old token field; decrypts the new token with an independent KMS configuration before creating the Vault client. |

Keep old and new environment names available while old and new binaries coexist. Old binaries continue reading the old field. New binaries load ciphertext into the primary_password field and replace it with plaintext at startup; the old password fields are no longer deserialized. The old field names are not aliases for the new encrypted fields.

When migrating password storage without rotating the upstream database or SMTP credential, encrypt the same existing credential into the new field. Rotating the upstream credential itself requires coordinating the upstream service's credential acceptance independently.

An absent optional encrypted password remains absent. Do not omit the encrypted field for a service that requires authentication. File-backed credentials and credentials embedded in connection URLs are outside this migration.

## Internal-service key rotation

The `primary_internal_api_key` field receives KMS ciphertext from the environment. Startup replaces that same field with the decrypted key; there is no separate decrypted-key field. A private initialization flag prevents authentication before decryption succeeds and cannot be set through configuration.

The flags are:

```text
ROUTER__INTERNAL_MERCHANT_ID_PROFILE_ID_AUTH__KEY_ROTATION_ENABLED
ROUTER__INTERNAL_MERCHANT_ID_PROFILE_ID_AUTH__USE_LEGACY_KEY_FOR_OUTBOUND
```

Both flags default to false. The new encrypted key is required when internal authentication is enabled or a legacy key/rotation mode is configured. The existing `ENABLED` setting still controls inbound internal authentication. Outgoing calls require initialized credentials even when the inbound setting is disabled.

| Rotation enabled | Use legacy key for outbound | Incoming authentication | Outgoing authentication |
|---|---|---|---|
| false | false | Decrypted new key only | Decrypted new key |
| true | false | Decrypted new key first, then legacy key | Decrypted new key |
| true | true | Decrypted new key first, then legacy key | Legacy key |
| false | true | Invalid configuration; startup fails | Invalid configuration |

Rotation mode requires both the encrypted new key and a nonempty legacy key. Invalid new ciphertext fails startup even when a legacy key exists. Empty/uninitialized keys cannot authenticate. Disabling rotation clears the legacy key from the runtime authentication settings.

The outbound switch is independent so requests do not fail during either rolling upgrade or rolling removal of legacy acceptance. No payment request is automatically retried with another authentication key.

### Staged rollout

1. Supply the encrypted new key and retain the existing legacy key. Set both flags to true for upgraded instances. Upgrade every sender and receiver; new receivers accept both keys, and upgraded senders remain compatible with old receivers.
2. After every receiver accepts both keys, leave `KEY_ROTATION_ENABLED=true` and roll out `USE_LEGACY_KEY_FOR_OUTBOUND=false` to every sender. During this rollout, receivers accept both old and new senders.
3. Once every sender uses the new key, roll out `KEY_ROTATION_ENABLED=false`. Keep `USE_LEGACY_KEY_FOR_OUTBOUND=false`. Remove the old deployment secret after the rollback window closes.

Every participating service must implement compatible receiver behavior. Upgrading only the router does not change a separately deployed receiver's authentication logic. Restart/redeploy instances after changing environment variables; these credentials are resolved during initialization.

## HashiCorp Vault token bootstrap

The Vault token cannot be decrypted using a Vault client that requires that same token to authenticate. Configure an independent KMS backend under `hc_vault.token_kms`.

For AWS KMS, the additional configuration names are:

```text
ROUTER__SECRETS_MANAGEMENT__SECRETS_MANAGER=hashi_corp_vault
ROUTER__SECRETS_MANAGEMENT__HC_VAULT__URL
ROUTER__SECRETS_MANAGEMENT__HC_VAULT__ENCRYPTED_TOKEN
ROUTER__SECRETS_MANAGEMENT__HC_VAULT__TOKEN_KMS__ENCRYPTION_MANAGER=aws_kms
ROUTER__SECRETS_MANAGEMENT__HC_VAULT__TOKEN_KMS__AWS_KMS__KEY_ID
ROUTER__SECRETS_MANAGEMENT__HC_VAULT__TOKEN_KMS__AWS_KMS__REGION
```

The binary needs the HashiCorp Vault feature and the chosen KMS provider feature. The workload identity must be able to decrypt the token with that KMS key. GCP and OCI can use the corresponding existing encryption-manager configuration when compiled in. `no_encryption` is rejected for the Vault token.

The independent token KMS client decrypts the Vault bootstrap token and the new KMS-only password/internal-key fields. Other application secrets fetched through the Vault backend keep their existing Vault lookup semantics.

## Authentication microservice API key

`ROUTER__MICRO_SERVICES__AUTHENTICATION_SERVICE__API_KEY` keeps its existing name. When the authentication-service configuration is present, this field must contain KMS ciphertext. Startup replaces it with the decrypted API key before any service calls. Missing, empty, or invalid ciphertext fails startup. When the service configuration is absent, no decryption is attempted.

Unlike the renamed password fields, this field has no old/new environment-name overlap for a rolling migration. Supply ciphertext only to upgraded instances; old instances still expect plaintext under the same name.
