# Superposition proxy workspace authorization

The `/v1/superposition/*` proxy uses a service credential upstream. A user's
configuration permission does not grant access to every workspace available to
that credential. Every proxy operation now requires an operator-provisioned
workspace mapping matching the authenticated tenant, organization, merchant and
profile. The existing read/write permission checks still apply.

`superposition.proxy_workspaces` defaults to an empty list. Existing deployments
therefore return 403 from the proxy until dedicated workspaces are provisioned.
The runtime configuration provider and its internal writes are unaffected.

Example configuration, using placeholders for a single profile:

```toml
[[superposition.proxy_workspaces]]
tenant_id = "tenant_example"
organization_id = "org_example"
merchant_id = "merchant_example"
profile_id = "pro_example"
superposition_org_id = "customer_configs"
workspace_id = "profile_example"
```

The Hyperswitch organization ID and the Superposition organization ID are
different namespaces. Requests must supply the mapped Superposition organization
and workspace in `x-org-id` and `x-workspace`. A mismatch returns 403 before an
upstream call; upstream requests use the identifiers from the mapping.

A workspace must belong exclusively to the mapped profile. Provision a clean
workspace containing only configuration and audit records that profile may see.
Do not copy a shared workspace's audit history or global runtime configuration.
All proxy operations, including default configuration reads and audit reads, are
workspace-wide unless they already have an additional context filter.

The configured runtime `superposition.org_id` / `superposition.workspace_id` pair
is always denied, even if a mapping is added for it. Duplicate mappings of any
upstream workspace also fail closed, including duplicates for the same owner.
Do not create aliases for the runtime workspace or share a mapped workspace
outside this configuration. Scope the service credential upstream to the minimum
required resources as well.

This is a conservative isolation proposal, not a migration of the existing
shared-workspace configuration UI. A UI that must edit shared runtime settings
needs a separate API enforcing ownership of individual configuration keys,
contexts and audit records; enabling a mapping to the shared workspace is not a
substitute for that API.

Regression tests cover unmapped targets, changes to either target header,
identity mismatches, ambiguous mappings, the shared runtime workspace, and
non-string context scope values. Build and test execution are required before
merging this draft.
