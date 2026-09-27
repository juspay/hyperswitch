// JSON structural-compare helpers shared by the combined-PML stability
// assertions (54-ServerIntegrationStability) and their command wrappers.
//
// Multi-replica deployments re-render the list ordering per request — both
// the top-level entry array and inner lists (e.g. eligible connectors) — so
// equality is checked on order-normalized copies instead of raw bodies.

// Deep-clones a JSON value with every object key and every array sorted, so
// two payloads that differ only in ordering compare equal.
export function normalizeJsonForCompare(value) {
  if (Array.isArray(value)) {
    return value
      .map(normalizeJsonForCompare)
      .sort((x, y) => (JSON.stringify(x) < JSON.stringify(y) ? -1 : 1));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, normalizeJsonForCompare(value[key])])
    );
  }
  return value;
}

// Walks two (normalized) JSON values and returns up to `limit` leaf paths
// where they differ, each as "path: actual <> expected"; empty when equal.
export function jsonDiffPaths(actual, expected, path = "$", limit = 5) {
  const diffs = [];
  const render = (value) => {
    const text = JSON.stringify(value);
    return text && text.length > 160 ? `${text.slice(0, 157)}...` : text;
  };

  const walk = (a, b, current) => {
    if (diffs.length >= limit) return;
    if (JSON.stringify(a) === JSON.stringify(b)) return;
    const bothObjects =
      a && b && typeof a === "object" && typeof b === "object";
    if (!bothObjects || Array.isArray(a) !== Array.isArray(b)) {
      diffs.push(`${current}: ${render(a)} <> ${render(b)}`);
      return;
    }
    if (Array.isArray(a)) {
      if (a.length !== b.length) {
        diffs.push(
          `${current}: array[len ${a.length}] <> array[len ${b.length}]`
        );
        return;
      }
      for (let i = 0; i < a.length && diffs.length < limit; i++) {
        walk(a[i], b[i], `${current}[${i}]`);
      }
      return;
    }
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const key of keys) {
      if (diffs.length >= limit) break;
      walk(a[key], b[key], `${current}.${key}`);
    }
  };

  walk(actual, expected, path);
  return diffs;
}
