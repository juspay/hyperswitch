# OCI KMS

A client for the OCI Vault KMS crypto endpoint, shared by Hyperswitch services that keep their
secrets or data keys under an OCI Vault key.

There is no official OCI Rust SDK, and the community ones don't cover the KMS crypto endpoint, so
this crate implements what an SDK would otherwise hide:

- **Authentication.** Inside Kubernetes, OKE Workload Identity: an ephemeral RSA key pair is
  generated in memory and the pod's service account token is exchanged for a short-lived OCI
  session token. Nothing credential-bearing is written to disk. The service account token and
  cluster CA are read from kubelet's default mount paths; pods that project them elsewhere set
  `OCI_KUBERNETES_SERVICE_ACCOUNT_TOKEN_PATH` and `OCI_KUBERNETES_SERVICE_ACCOUNT_CERT_PATH`
  (the latter is the variable Oracle's SDKs read). Outside Kubernetes (local
  development, CI), the `oci` CLI's `~/.oci/config` (`OCI_CLI_CONFIG_FILE`, `OCI_CLI_PROFILE`).
- **Request signing**, OCI Signature v1.
- **Timeouts and retries**, with jittered exponential backoff on transport errors, 429 and 5xx.

It has no dependency on other Hyperswitch crates, so services outside this workspace can use it.

## Usage

```rust,ignore
let client = oci_kms::OciKmsClient::new(&oci_kms::OciKmsConfig {
    vault_crypto_endpoint: "https://<vault>-crypto.kms.<region>.oci.oraclecloud.com".into(),
    key_id: "ocid1.key.oc1...".into(),
})?;

// Config secrets: ciphertext produced once with `oci kms crypto encrypt`.
let plaintext: Vec<u8> = client.decrypt(ciphertext).await?;

// Envelope encryption: a fresh AES-256 data key, plus the same key wrapped by the vault key.
let data_key = client.generate_data_key().await?;
```

Services that need clock and entropy reads to go through their own instrumented functions (for
example, to make them replayable) pass an `Environment` to `OciKmsClient::with_environment`.

## Testing

Unit tests run with `cargo test -p oci_kms`. Tests against a real vault are `#[ignore]`d and read
their configuration from the environment; see the `live` module in `src/client.rs`.
