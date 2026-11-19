import { createHash } from 'node:crypto';

/** Stable JSON: object keys sorted recursively, arrays keep order. */
export function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    const obj = value as Record<string, unknown>;
    const parts = Object.keys(obj)
      .sort()
      .filter((k) => obj[k] !== undefined)
      .map((k) => `${JSON.stringify(k)}:${canonicalJson(obj[k])}`);
    return `{${parts.join(',')}}`;
  }
  return JSON.stringify(value) ?? 'null';
}

/**
 * sha256 over the canonical request plus the resolved payment id. A caller-supplied `routing` is
 * excluded (this service overwrites it anyway), as is body `payment_id` (the resolved id is hashed
 * instead, so Idempotency-Key vs body payment_id does not change the fingerprint).
 * Only the hash is ever stored: the body may contain card data.
 */
export function requestFingerprint(paymentId: string, body: Record<string, unknown>): string {
  const { routing: _routing, payment_id: _paymentId, ...rest } = body;
  return createHash('sha256').update(canonicalJson({ paymentId, body: rest })).digest('hex');
}
