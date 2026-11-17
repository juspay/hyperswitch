/**
 * Classification of Hyperswitch AttemptStatus values (wire format is
 * snake_case, serde rename_all in common_enums). The mapping and its reasons
 * are documented in docs/ATTEMPT_OUTCOMES.md.
 */
export const OUTCOME_CLASSES = [
  'success',
  'technical_failure',
  'issuer_decline',
  'integrity_failure',
  'unresolved',
  'ignored',
] as const;
export type OutcomeClass = (typeof OUTCOME_CLASSES)[number];
export type ClassCounts = Record<OutcomeClass, number>;

export function emptyCounts(): ClassCounts {
  return { success: 0, technical_failure: 0, issuer_decline: 0, integrity_failure: 0, unresolved: 0, ignored: 0 };
}

/** 'failure' is resolved to technical_failure / issuer_decline by error code. */
type StatusRule = Exclude<OutcomeClass, 'technical_failure' | 'issuer_decline'> | 'failure';

export const ATTEMPT_STATUS_TABLE = {
  charged: 'success',
  authorized: 'success',
  partial_charged: 'success',
  partially_authorized: 'success',
  partial_charged_and_chargeable: 'success',

  started: 'unresolved',
  pending: 'unresolved',
  authorizing: 'unresolved',
  authentication_pending: 'unresolved',
  authentication_successful: 'unresolved',
  confirmation_awaited: 'unresolved',
  payment_method_awaited: 'unresolved',
  device_data_collection_pending: 'unresolved',
  unresolved: 'unresolved',
  cod_initiated: 'unresolved',
  expired: 'unresolved',

  authentication_failed: 'ignored',
  router_declined: 'ignored',
  voided: 'ignored',
  voided_post_charge: 'ignored',
  void_initiated: 'ignored',
  capture_initiated: 'ignored',
  capture_failed: 'ignored',
  void_failed: 'ignored',
  auto_refunded: 'ignored',
  capture_review: 'ignored',

  failure: 'failure',
  authorization_failed: 'failure',
  // Own class: excluded from reliability unless connector responsibility is established.
  integrity_failure: 'integrity_failure',
} as const satisfies Record<string, StatusRule>;

export type AttemptStatus = keyof typeof ATTEMPT_STATUS_TABLE;

export interface Classification {
  outcome: OutcomeClass;
  /** Set for a status string this table does not know (e.g. a newer Hyperswitch); the caller must log it. */
  warning: string | null;
}

/**
 * Unknown status strings are `ignored` (never a failure) AND reported through `warning`,
 * so they are not silent. The engine stays pure: the adapter does the logging.
 */
export function classifyAttempt(
  status: string,
  errorCode: string | null | undefined,
  issuerDeclineCodes: ReadonlySet<string>,
): Classification {
  const rule: StatusRule | undefined = (ATTEMPT_STATUS_TABLE as Record<string, StatusRule>)[status];
  if (rule === undefined) {
    return { outcome: 'ignored', warning: `unknown attempt status "${status}" classified as ignored` };
  }
  if (rule !== 'failure') return { outcome: rule, warning: null };
  // MVP decision: a Failure/AuthorizationFailed with no or unlisted error code is a technical failure.
  const outcome = errorCode != null && issuerDeclineCodes.has(errorCode) ? 'issuer_decline' : 'technical_failure';
  return { outcome, warning: null };
}
