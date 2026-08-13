function targetHeaders(input, target) {
  return { ...(input.target_headers?.[target] || {}) };
}

export function modularHeaders(input, clientSecret) {
  return {
    ...targetHeaders(input, "modular-pm"),
    Authorization: clientSecret
      ? `publishable-key=${input.merchant.publishable_key},client-secret=${clientSecret}`
      : `api-key=${input.merchant.merchant_api_key}`,
    "x-profile-id": input.merchant.profile_id,
    "content-type": "application/json",
  };
}

export function apiKeyHeaders(input) {
  return {
    ...targetHeaders(input, "router"),
    "api-key": input.merchant.merchant_api_key,
    "content-type": "application/json",
  };
}
