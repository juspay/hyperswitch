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
let merchantIdM3;
let jwtMerchantAdminM1;

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
      merchantIdM3 = globalState.get("merchantIdM3");
      jwtMerchantAdminM1 = globalState.get("jwtMerchantAdminM1");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("M1 queries payments metrics with M2's merchant_id spoofed into the filter body", () => {
    // M1's dashboard user is org_admin (signup_with_merchant_id grants
    // ROLE_ID_ORGANIZATION_ADMIN, core/user.rs:96-103) - but org_admin
    // doesn't mean "sees every merchant everywhere": M1 and M2 are each
    // created in their OWN brand-new organization (00-Setup.cy.js's M1
    // signup and M2's merchantCreateCallTest both resolve org_id via
    // CreateOrValidateOrganization::new(None, ...), admin.rs:506-523 -
    // neither passes an existing organization_id, so each gets a fresh
    // org). They were never siblings under one org to begin with.
    //
    // Even setting that aside, this specific endpoint
    // (get_merchant_payment_metrics, JWTAuth{MerchantAnalyticsRead})
    // builds its AuthInfo strictly from the JWT's own merchant_id
    // (to_merchant_level_auth_info, platform.rs:373-395) and never
    // expands to other merchants in the org regardless of role -
    // org-wide access only exists on the separate
    // get_org_payment_metrics endpoint (OrganizationAnalyticsRead +
    // PlatformOrgAdminAuth). validate_analytics_scope
    // (analytics_validator.rs:11-35) rejects any filters.merchant_id
    // that isn't in that single-merchant list with AccessForbiddenError
    // (401) - confirmed live. So 401 here is the correct, expected
    // denial, not a loose "some 4xx" catch-all.
    cy.ompQueryPaymentMetrics(
      { type: "jwt", token: jwtPA },
      metricsRequest({ merchant_id: [merchantIdM2] }),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
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

  context(
    "org_admin vs merchant_admin scope (M1 and M3, same organization)",
    () => {
      // M3 (00-Setup.cy.js) is a genuine sibling of M1 - created with
      // organization_id explicitly set to M1's own org, unlike M2 above
      // (which is a wholly separate org). jwtMerchantAdminM1 is a second,
      // separately-provisioned user with role_id "merchant_admin" (Merchant
      // entity_type), invited and logged in via the real /user/invite_multiple
      // + signin flow - not M1's own org_admin session. These are two
      // independent prerequisites (M3 only needs merchant creation; the
      // merchant_admin user additionally needs a real email-invite accept
      // this suite can't automate in every environment), so they're gated
      // separately below rather than as one combined guard - the org_admin
      // tests should still run even when the merchant_admin bootstrap
      // couldn't complete.
      context("M3 required", () => {
        before(function () {
          if (!merchantIdM3) {
            cy.task(
              "cli_log",
              "Skipping: M3 was not provisioned in 00-Setup.cy.js."
            );
          }
        });

        beforeEach(function () {
          if (!merchantIdM3) this.skip();
        });

        it("org_admin sees a sibling merchant via the ORG-level endpoint (positive)", () => {
          // This is the legitimate org-wide access path -
          // PlatformOrgAdminAuth or JWT(OrganizationAnalyticsRead), AuthInfo
          // built from to_org_level_auth_info() (analytics.rs:642), not
          // from the JWT's own merchant_id. An org_admin's JWT querying a
          // genuinely-same-org sibling here should succeed.
          cy.ompQueryOrgPaymentMetrics(
            { type: "jwt", token: jwtPA },
            metricsRequest({ merchant_id: [merchantIdM3] }),
            globalState
          ).then((res) => {
            expect(res.status).to.eq(200);
          });
        });

        it("org_admin is still denied on the MERCHANT-level endpoint for a same-org sibling", () => {
          // The sharper version of the cross-org test above: this time M3
          // IS a genuine sibling in M1's own org, isolating the variable
          // the first test couldn't - get_merchant_payment_metrics never
          // expands to sibling merchants regardless of org_admin role,
          // confirmed by source (to_merchant_level_auth_info only ever
          // uses the JWT's own merchant_id). Same org membership doesn't
          // matter for this specific endpoint; only the endpoint choice
          // does.
          cy.ompQueryPaymentMetrics(
            { type: "jwt", token: jwtPA },
            metricsRequest({ merchant_id: [merchantIdM3] }),
            globalState
          ).then((res) => {
            expect(res.status).to.eq(401);
          });
        });
      });

      context("M3 and the merchant_admin user required", () => {
        before(function () {
          if (!merchantIdM3 || !jwtMerchantAdminM1) {
            cy.task(
              "cli_log",
              "Skipping: M3 and/or the merchant_admin user were not " +
                "provisioned in 00-Setup.cy.js (see that file's 'M3 and a " +
                "merchant_admin user' context for why this might fail - " +
                "e.g. this environment requires a real email invite accept, " +
                "which this bootstrap flow doesn't support)."
            );
          }
        });

        beforeEach(function () {
          if (!merchantIdM3 || !jwtMerchantAdminM1) this.skip();
        });

        it("merchant_admin queries its own merchant's metrics (positive)", () => {
          cy.ompQueryPaymentMetrics(
            { type: "jwt", token: jwtMerchantAdminM1 },
            metricsRequest({ merchant_id: [merchantIdM1] }),
            globalState
          ).then((res) => {
            expect(res.status).to.eq(200);
          });
        });

        it("merchant_admin is denied on a same-org sibling merchant's metrics", () => {
          cy.ompQueryPaymentMetrics(
            { type: "jwt", token: jwtMerchantAdminM1 },
            metricsRequest({ merchant_id: [merchantIdM3] }),
            globalState
          ).then((res) => {
            expect(res.status).to.eq(401);
          });
        });

        it("merchant_admin is denied on the ORG-level endpoint entirely", () => {
          // merchant_admin's role has entity_type Merchant, not
          // Organization - it shouldn't hold OrganizationAnalyticsRead at
          // all. Asserting a 4xx range rather than an exact code here:
          // unlike the MerchantAnalyticsRead paths above (confirmed exact
          // IR via validate_analytics_scope), the exact rejection shape
          // for a role that fails the permission check itself (rather
          // than the merchant_id filter check) hasn't been independently
          // confirmed - tighten this once seen live.
          cy.ompQueryOrgPaymentMetrics(
            { type: "jwt", token: jwtMerchantAdminM1 },
            metricsRequest({ merchant_id: [merchantIdM1] }),
            globalState
          ).then((res) => {
            expect(res.status).to.be.within(400, 499);
          });
        });
      });
    }
  );

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
