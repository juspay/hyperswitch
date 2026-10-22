# Distributed load-test deployment

Build and publish the console, cloud control-plane, and worker images. The cloud
image is `loadtest/console/Dockerfile.cloud`; set `CONSOLE_IMAGE` to the base
console image when building it. Create the two referenced Secrets, then apply
`control-plane.yaml` and `workers.yaml`. Expose the control
plane through the platform HTTPS ingress. Keep worker traffic outbound-only.

The manifests intentionally leave cloud identity configuration to the target
cluster. Create the `loadtest-control-plane` ServiceAccount and bind it through
GKE Workload Identity to an IAM service account with only the required AlloyDB
failover permission. Grant that Kubernetes ServiceAccount permission to patch
the configured card-vault Deployment. Attach gcloud and kubectl identity only to the control-plane service
account. Load workers require access to the tested APIs and no infrastructure
mutation permissions.

Start with four workers at 250 TPS each, then calibrate `LOADTEST_WORKER_TPS`,
CPU, memory and VU capacity from a sustained benchmark before raising limits.
