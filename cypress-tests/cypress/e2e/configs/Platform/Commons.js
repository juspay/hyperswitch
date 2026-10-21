// Shared error expectations for Platform payment endpoints
// (/payments/platform/list and /payments/platform/filter). These are
// connector-agnostic platform endpoint expectations (identical for every
// connector); consume them from Platform specs instead of inlining the
// error payloads.

// Auth negative: Authorization header omitted entirely
export const platformPaymentsMissingAuthorizationError = {
  status: 400,
  error: {
    type: "invalid_request",
    code: "IR_04",
    message: "Missing required param: Authorization",
  },
};

// Auth negative: malformed JWT bearer token
export const platformPaymentsInvalidJwtError = {
  status: 401,
  error: {
    type: "invalid_request",
    code: "IR_17",
    message: "Access forbidden, invalid JWT token was used",
  },
};

// Auth negative: standard (non-platform) merchant JWT used on a
// platform endpoint
export const platformPaymentsStandardMerchantJwtError = {
  status: 401,
  error: {
    type: "invalid_request",
    code: "IR_01",
    message: "API key not provided or invalid API key used",
  },
};

// Query validation negatives for the platform payment list; these fail
// query deserialization and return a plain-text body
export const platformPaymentsListLimitZeroError = {
  status: 400,
  rawError:
    "Query deserialize error: list limit 0 is invalid, it must be between 1 and 100",
};

export const platformPaymentsListLimitAboveMaxError = {
  status: 400,
  rawError:
    "Query deserialize error: list limit 101 is invalid, it must be between 1 and 100",
};

export const platformPaymentsListOffsetAboveMaxError = {
  status: 400,
  rawError:
    "Query deserialize error: list offset 20001 is invalid, it must be at most 20000",
};

export const platformPaymentsListInvalidStatusError = {
  status: 400,
  rawError:
    "Query deserialize error: Invalid value 'not_a_status': Matching variant not found",
};
