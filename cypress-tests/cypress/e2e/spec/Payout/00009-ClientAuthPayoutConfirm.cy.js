import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

const executionEndpoints = [
  { name: "create", method: "POST", path: "/create" },
  { name: "update", method: "PUT", path: "" },
  { name: "confirm", method: "POST", path: "/confirm" },
  { name: "cancel", method: "POST", path: "/cancel" },
  { name: "fulfill", method: "POST", path: "/fulfill" },
];

const retrievalEndpoints = [
  { name: "retrieve" },
  { name: "sync", qs: { force_sync: true } },
];

// An invalid JWT is sufficient to distinguish the auth paths: execution must
// report the missing API key (IR_01), while retrieve and dashboard APIs validate JWTs.
const invalidJwt = "payout-execution-auth-regression";

function getAuthHeaders(authMode) {
  switch (authMode) {
    case "publishable-key":
      return { "api-key": globalState.get("publishableKey") };
    case "jwt":
      return { Authorization: `Bearer ${invalidJwt}` };
    case "sdk":
      return {
        Authorization: btoa(
          `profile_id=${globalState.get("profileId")},publishable_key=${globalState.get("publishableKey")},client_secret=${globalState.get("payoutClientSecret") || "payout_client_secret"}`
        ),
      };
    default:
      return {};
  }
}

describe("[Payout] Execution Authentication", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      shouldContinue = Boolean(globalState.get("payoutsExecution"));
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
    payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
  });

  context("[Payout] Execution requires an API key", () => {
    ["publishable-key", "jwt", "sdk", "no-credentials"].forEach((authMode) => {
      executionEndpoints.forEach((endpoint) => {
        it(`rejects-${authMode}-on-${endpoint.name}`, () => {
          const payoutId =
            globalState.get("payoutID") || "payout_auth_scope_regression";
          const path =
            endpoint.name === "create"
              ? endpoint.path
              : `/${payoutId}${endpoint.path}`;

          cy.request({
            method: endpoint.method,
            url: `${globalState.get("baseUrl")}/payouts${path}`,
            headers: {
              "Content-Type": "application/json",
              ...getAuthHeaders(authMode),
            },
            qs: endpoint.qs,
            body:
              endpoint.method === "GET"
                ? undefined
                : {
                    ...payoutBody,
                    confirm: false,
                    client_secret:
                      globalState.get("payoutClientSecret") ||
                      "payout_client_secret",
                  },
            failOnStatusCode: false,
          }).then((response) => {
            expect(response.status).to.equal(401);
            expect(response.body.error.code).to.equal("IR_01");
            expect(response.body.error.message).to.equal(
              "API key not provided or invalid API key used"
            );
          });
        });
      });
    });
  });

  context("[Payout] Retrieve keeps API-key and JWT authentication", () => {
    retrievalEndpoints.forEach((endpoint) => {
      it(`validates-jwt-on-${endpoint.name}`, () => {
        const payoutId =
          globalState.get("payoutID") || "payout_auth_scope_regression";

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payouts/${payoutId}`,
          headers: getAuthHeaders("jwt"),
          qs: endpoint.qs,
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(401);
          expect(response.body.error.code).to.equal("IR_17");
        });
      });

      ["publishable-key", "sdk", "no-credentials"].forEach((authMode) => {
        it(`rejects-${authMode}-on-${endpoint.name}`, () => {
          const payoutId =
            globalState.get("payoutID") || "payout_auth_scope_regression";

          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/payouts/${payoutId}`,
            headers: getAuthHeaders(authMode),
            qs: endpoint.qs,
            failOnStatusCode: false,
          }).then((response) => {
            expect(response.status).to.equal(401);
            expect(response.body.error.code).to.equal("IR_01");
            expect(response.body.error.message).to.equal(
              "API key not provided or invalid API key used"
            );
          });
        });
      });
    });
  });

  context("[Payout] Dashboard keeps JWT authentication", () => {
    ["/list", "/v2/filter"].forEach((path) => {
      it(`validates-jwt-on-${path}`, () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payouts${path}`,
          headers: getAuthHeaders("jwt"),
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(401);
          expect(response.body.error.code).to.equal("IR_17");
        });
      });
    });
  });

  context(
    "[Payout] Confirm rejects client auth and accepts merchant auth",
    () => {
      beforeEach(function () {
        if (
          !utils.CONNECTOR_LISTS.INCLUDE.WALLET_PAYOUT.includes(
            globalState.get("connectorId")
          )
        ) {
          this.skip();
        }
      });

      it("create-payout-for-auth-confirm-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["Create"];

        cy.createConfirmPayoutTest(payoutBody, data, false, true, globalState);
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      [
        "ConfirmClientAuth",
        "ConfirmClientAuthRestrictedFields",
        "ConfirmClientAuthMultipleRestrictedFields",
        "ConfirmClientAuthMissingClientSecret",
      ].forEach((key) => {
        it(`rejects-${key}`, () => {
          const data = utils.getConnectorDetails(
            globalState.get("connectorId")
          )["wallet_pm"][key];

          cy.confirmPayoutCallTest({}, data, true, globalState);
        });
      });

      it("retrieve-payout-after-client-auth-errors-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["RetrieveAfterClientAuthErrors"];

        cy.retrievePayoutCallTest(globalState, data);
      });

      it("merchant-auth-confirm-with-amount-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["ConfirmMerchantAuth"];

        cy.confirmPayoutCallTest({}, data, false, globalState);
      });

      it("retrieve-payout-after-merchant-auth-confirm-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["RetrieveAfterConfirm"];

        cy.retrievePayoutCallTest(globalState, data);
      });
    }
  );
});
