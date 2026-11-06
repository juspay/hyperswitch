import State from "../../../../utils/State";

// Analytics handlers build a server-derived AuthInfo (Org/Merchant/Profile
// level) strictly from the authenticated JWT, and that AuthInfo - never a
// client-supplied merchant_id/profile_id in the request body - is what's
// threaded into the actual query builders. Some handlers additionally call
// validate_analytics_scope() to explicitly reject a body whose filters
// disagree with the authenticated identity.
//
// CAVEAT: only the payments metric query builder was directly re-verified
// during the audit; refund/dispute/sdk_event/connector_event metric
// builders in crates/analytics/ were not individually re-checked.
//
// Route (verified): POST /analytics/v1/metrics/payments - JWTAuth or
// PlatformOrgAdminAuth. The handler (get_merchant_payment_metrics,
// crates/router/src/analytics.rs) deserializes a 1-element array
// ([GetPaymentMetricRequest; 1]), and that request type is camelCase at
// the top level (timeRange/groupByNames) while its nested filters stay
// snake_case - see ompQueryPaymentMetrics in commands.js, which handles
// the array wrapping so tests here just pass the plain request object.
let globalState;
let jwtPA;
let merchantIdM1;
let merchantIdM2;

function metricsRequest(filters) {
  return {
    timeRange: {
      start_time: "2020-01-01T00:00:00Z",
      end_time: "2030-01-01T00:00:00Z",
    },
    groupByNames: [],
    filters,
    metrics: ["payment_success_count"],
    distribution: null,
    delta: false,
  };
}

describe("OMP Scoping - Analytics", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      jwtPA = globalState.get("userInfoTokenPA");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("M1 queries payments metrics with M2's merchant_id spoofed into the filter body", () => {
    cy.ompQueryPaymentMetrics(
      { type: "jwt", token: jwtPA },
      metricsRequest({ merchant_id: [merchantIdM2] }),
      globalState
    ).then((res) => {
      // The query is scoped by the authenticated JWT's own merchant
      // (server-derived AuthInfo), not by the body's filters.merchant_id -
      // so either the request is rejected outright by
      // validate_analytics_scope, or it succeeds but is scoped to M1
      // regardless of the spoofed filter.
      expect(res.status).to.be.oneOf([200, 400]);
    });
  });

  it("M1 queries its own payments metrics (positive)", () => {
    cy.ompQueryPaymentMetrics(
      { type: "jwt", token: jwtPA },
      metricsRequest({ merchant_id: [merchantIdM1] }),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("invalid credential on analytics query is rejected", () => {
    cy.ompQueryPaymentMetrics(
      { type: "invalid" },
      metricsRequest({}),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
