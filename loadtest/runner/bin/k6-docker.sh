#!/usr/bin/env bash
set -euo pipefail

workspace_root="$(git rev-parse --show-toplevel)"
docker_args=(
  run --rm
  --network host
  --user "$(id -u):$(id -g)"
  --env K6_INPUT
  --volume "$workspace_root:$workspace_root"
  --workdir "$PWD"
)

# The runner applies the generated k6 affinity to this process. Propagate that
# affinity to the container, since pinning only the Docker client is ineffective.
if affinity="$(taskset -pc "$$" 2>/dev/null)"; then
  affinity="${affinity##*: }"
  docker_args+=(--cpuset-cpus "$affinity")
fi

exec docker "${docker_args[@]}" "${K6_DOCKER_IMAGE:-grafana/k6:latest}" "$@"
