import type { AttemptView } from '../hyperswitch/client';

/**
 * What Hyperswitch actually did, kept next to what we asked for. Hyperswitch treats the routing
 * override as a preference, not a guarantee (a disabled or unknown connector is silently replaced,
 * and auto-retry can continue past the requested list), so the two must be reported separately.
 */
export interface ExecutionRecord {
  /** Connectors in the override we sent, in the order we sent them; null when no override was sent. */
  requestedOrder: string[] | null;
  /** Payment-level `connector` and `status` from Hyperswitch's response. */
  finalConnector: string | null;
  finalStatus: string | null;
  /** Attempts as reported; null when they could not be read. Listed alphabetically, not by time. */
  attempts: { attemptId: string | null; connector: string; status: string }[] | null;
  /** Hyperswitch does not guarantee the order of `attempts[]`, so no attempt ordering is claimed. */
  attemptOrderEstablished: false;
  /** Connectors that handled the payment but were not in the requested list. */
  usedOutsideRequested: string[];
  /** Whether the final connector is the first one we requested; null when it cannot be judged. */
  finalIsFirstRequested: boolean | null;
  notes: string[];
}

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);

export function describeExecution(
  requestedOrder: readonly string[] | null,
  responseBody: Record<string, unknown>,
  attempts: readonly AttemptView[] | null,
): ExecutionRecord {
  const finalConnector = str(responseBody['connector']);
  const finalStatus = str(responseBody['status']);
  const requested = requestedOrder === null ? null : [...requestedOrder];
  const notes: string[] = ['attempt order is not established: Hyperswitch does not guarantee the order of attempts[]'];

  const listed =
    attempts === null
      ? null
      : attempts
          .map((a) => ({ attemptId: a.attemptId, connector: a.connector, status: a.status }))
          .sort((x, y) => (x.connector < y.connector ? -1 : x.connector > y.connector ? 1 : (x.attemptId ?? '') < (y.attemptId ?? '') ? -1 : 1));
  if (listed === null) notes.push('attempts could not be read from Hyperswitch');

  const handled = new Set<string>();
  for (const a of listed ?? []) handled.add(a.connector);
  if (finalConnector !== null) handled.add(finalConnector);
  const usedOutsideRequested = requested === null ? [] : [...handled].filter((c) => !requested.includes(c)).sort();

  if (requested === null) notes.push('no routing override was sent; Hyperswitch chose the connector itself');
  if (usedOutsideRequested.length > 0) {
    notes.push(`connectors used outside the requested list: ${usedOutsideRequested.join(', ')} (the override is a preference, not a guarantee)`);
  }
  if (requested !== null && requested.length > 0 && listed !== null && listed.length > 0 && !listed.some((a) => a.connector === requested[0])) {
    notes.push('the first requested connector does not appear in the attempts');
  }

  const finalIsFirstRequested =
    requested === null || requested.length === 0 || finalConnector === null ? null : finalConnector === requested[0];

  return {
    requestedOrder: requested,
    finalConnector,
    finalStatus,
    attempts: listed,
    attemptOrderEstablished: false,
    usedOutsideRequested,
    finalIsFirstRequested,
    notes,
  };
}
