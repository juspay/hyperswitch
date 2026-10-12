# Hyperswitch development stack shutdown

Target: project `hyperswitch-dev`, region `asia-south1`, GKE cluster `hyperswitch-dev-gke-v1`.

The user authorized stopping the five AlloyDB instances, 26 application/monitoring/ingress workloads, seven node pools, and five dedicated proxy/bastion VMs. Security-review VMs are excluded. Databases, disks, workload definitions, and the GKE control plane are retained.

Restore records:

- `before-shutdown.json`: original replica counts, database activation policies, node autoscaling settings, and VM state.
- `node-group-sizes.json`: original per-zone node group sizes.
- `proxy-autoscalers.json`: original Envoy/Squid autoscaling policies.
- `proxy-managed-groups.json`: original proxy redistribution and standby policies.

Restore only when requested. Start the bastion/proxy VMs and primary databases first, then read pools. Re-enable GKE autoscaling with the saved bounds and restore the saved workload replica counts. Restore proxy automatic redistribution and autoscaling after their VMs are running. Check database availability and deployment readiness before starting another load test.

The shutdown leaves storage, load balancers, and the GKE control plane allocated; it is not resource deletion or a guarantee of zero project billing.
