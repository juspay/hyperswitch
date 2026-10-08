export function workerFailure(code, result, summary, signal) {
  if (code === 0) return null;
  if (result?.error) return result.error;
  if (code === 99) {
    const failed = Object.entries(summary?.metrics || {}).flatMap(([name, metric]) =>
      Object.entries(metric.thresholds || {}).filter(([, check]) => check.ok === false)
        .map(([expression]) => `${name} (${expression})`));
    return `Load completed but k6 thresholds failed${failed.length ? ': ' + failed.join(', ') : ''}. Open Results → Responses for recorded errors.`;
  }
  return `Worker exited ${code ?? signal}`;
}
