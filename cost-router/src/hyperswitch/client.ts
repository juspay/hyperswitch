import type { Candidate } from '../engine/types';

/** One attempt as read from a Hyperswitch payment response (`PaymentAttemptResponse`, api_models/payments.rs). */
export interface AttemptView {
  attemptId: string | null;
  connector: string;
  status: string;
  errorCode: string | null;
}

export interface HyperswitchResponse {
  status: number;
  body: Record<string, unknown>;
}

export interface HyperswitchClient {
  createPayment(body: Record<string, unknown>): Promise<HyperswitchResponse>;
  /**
   * [V live run, see docs/INTEGRATION_RESULTS.md] the CREATE response does not carry `attempts`;
   * `GET /payments/{id}?expand_attempts=true` does. Attempts have attempt_id, status, connector and
   * error_code, no merchant_connector_id, and their array order is not guaranteed.
   */
  retrieveAttempts(paymentId: string): Promise<AttemptView[]>;
}

/**
 * THE ONLY PLACE the routing override is built.
 *
 * Shape: {"type":"priority","data":[{"connector":"<name>","merchant_connector_id":"<id>"}]}
 * - [V] serde shape: StraightThroughAlgorithm in crates/api_models/src/routing.rs.
 * - [V live run] POST /payments accepts it and the first attempt follows the list order. It is a
 *   preference, not a guarantee: a disabled/unknown connector is silently replaced, an entry without
 *   merchant_connector_id is ignored, and with profile auto-retry on, retries can go past the list.
 */
export function buildPriorityRouting(ordered: readonly Candidate[]): Record<string, unknown> {
  return {
    type: 'priority',
    data: ordered.map((c) => ({ connector: c.connector, merchant_connector_id: c.merchantConnectorId })),
  };
}

/** Tolerant reader: skips entries without a connector or status instead of throwing. */
export function parseAttempts(body: Record<string, unknown>): AttemptView[] | null {
  const raw = body['attempts'];
  if (!Array.isArray(raw)) return null;
  const out: AttemptView[] = [];
  for (const item of raw as unknown[]) {
    if (typeof item !== 'object' || item === null) continue;
    const a = item as Record<string, unknown>;
    if (typeof a['connector'] !== 'string' || typeof a['status'] !== 'string') continue;
    out.push({
      attemptId: typeof a['attempt_id'] === 'string' ? a['attempt_id'] : null,
      connector: a['connector'],
      status: a['status'],
      errorCode: typeof a['error_code'] === 'string' ? a['error_code'] : null,
    });
  }
  return out;
}

export interface HttpClientOptions {
  baseUrl: string;
  apiKey: string;
  fetchImpl?: typeof fetch;
}

export class HttpHyperswitchClient implements HyperswitchClient {
  private readonly fetchImpl: typeof fetch;

  constructor(private readonly opts: HttpClientOptions) {
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async createPayment(body: Record<string, unknown>): Promise<HyperswitchResponse> {
    return this.call('/payments', { method: 'POST', body: JSON.stringify(body) });
  }

  async retrieveAttempts(paymentId: string): Promise<AttemptView[]> {
    const res = await this.call(`/payments/${encodeURIComponent(paymentId)}?expand_attempts=true`, { method: 'GET' });
    return parseAttempts(res.body) ?? [];
  }

  private async call(path: string, init: { method: string; body?: string }): Promise<HyperswitchResponse> {
    const res = await this.fetchImpl(this.opts.baseUrl.replace(/\/+$/, '') + path, {
      ...init,
      headers: { 'content-type': 'application/json', 'api-key': this.opts.apiKey },
    });
    const text = await res.text();
    let body: Record<string, unknown>;
    try {
      body = text ? (JSON.parse(text) as Record<string, unknown>) : {};
    } catch {
      body = { raw: text };
    }
    return { status: res.status, body };
  }
}
