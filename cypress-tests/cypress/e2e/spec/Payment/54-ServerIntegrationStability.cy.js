import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

// Superposition context the `system.payment_integration_type` config is
// dimensioned on (see dimension_config.rs: processor + provider merchant id).
const merchantContext = () => ({
  provider_merchant_id: globalState.get("merchantId"),
  processor_merchant_id: globalState.get("merchantId"),
});

// PR #14174 + hyperswitch-cloud#23422: the PM vault session and the combined
// payment-method list are pinned in Redis per payment, so the SDK's own list
// call, the standalone session-token route and the server-integration
// enrichment must agree for the same payment. The header/validation contract
// itself is covered by 54-ServerIntegrationHeader — this spec only adds the
// stability cases from the issue.
describe("Server integration — vault session and combined PML stability per payment", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      specShouldSkip = shouldIncludeConnector(
        globalState.get("connectorId"),
        CONNECTOR_LISTS.INCLUDE.SERVER_INTEGRATION_HEADER
      );
      if (specShouldSkip) {
        cy.task(
          "cli_log",
          "Connector not in SERVER_INTEGRATION_HEADER list — skipping ServerIntegrationStability spec"
        );
        return;
      }
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping ServerIntegrationStability spec"
        );
        specShouldSkip = true;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup superposition config + flush global state", () => {
    cy.deleteSuperpositionConfig(globalState, merchantContext());
    // Safety net: a failed re-enable step must not leave the connector down
    // for later specs.
    if (globalState.get("connectorDisabled")) {
      cy.connectorDisabledCallTest(globalState, false);
    }
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "vault session and combined PML are stable per payment (issue cases 2-5)",
    () => {
      // Saves a card for a fresh customer via a create-and-confirm
      // (same mechanism 14-SaveCardFlow uses).
      const saveCardForCustomer = () => {
        const saveCardData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          saveCardData,
          "no_three_ds",
          "automatic",
          globalState
        );
      };

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Enable server integration via superposition", () => {
        cy.setSuperpositionConfig(
          globalState,
          "system.payment_integration_type",
          "client_and_server",
          merchantContext()
        );
        cy.waitForConfigPropagation(
          globalState,
          200,
          "stability_integration_type",
          { "X-Integration-Type": "server" }
        );
      });

      it("Save a card for the customer (setup payment)", () => {
        saveCardForCustomer();
      });

      it("Create the test intent with X-Integration-Type: server (issue case 1)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ServerIntegrationCreate"];

        // Captured inline (not via the command): the enriched sections of this
        // response seed the cross-route equality assertions below.
        const body = {
          ...JSON.parse(JSON.stringify(fixtures.createPaymentBody)),
          ...data.Request,
          customer_id: globalState.get("customerId"),
          profile_id: globalState.get("profileId"),
          authentication_type: "no_three_ds",
          capture_method: "automatic",
        };
        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("apiKey"),
            "X-Integration-Type": "server",
          },
          failOnStatusCode: false,
          body,
        }).then((response) => {
          expect(response.status, "status_code").to.equal(200);
          globalState.set("clientSecret", response.body.client_secret);
          globalState.set("paymentID", response.body.payment_id);

          const list = response.body.payment_method_list;
          expect(list, "payment_method_list").to.be.an("object").and.to.not.be
            .null;
          expect(
            list.customer_payment_methods,
            "customer_payment_methods has the saved card"
          )
            .to.be.an("array")
            .and.to.have.length(1);
          const saved = list.customer_payment_methods[0];
          expect(saved.payment_token, "payment_token").to.be.a("string").and.to
            .not.be.empty;
          globalState.set("basePaymentToken", saved.payment_token);
          globalState.set("enrichedCreateCpm", saved);
          globalState.set(
            "enrichedCreateEnabled",
            list.payment_methods_enabled
          );

          // Stash the session authorization only when this environment mints
          // one (vault service wired); the standalone call seeds it otherwise.
          const authorization =
            response.body.session_tokens?.vault_details?.vault_data
              ?.sdk_authorization ?? null;
          if (authorization) {
            globalState.set("sdkVaultAuthorization", authorization);
          }
        });
      });

      it("Combined PML is byte-identical across calls (issue case 2)", () => {
        cy.clientPaymentMethodsListCall(globalState, {
          storeAs: "sdkPmlBaseline",
        });
        cy.clientPaymentMethodsListCall(globalState, {
          compareTo: "sdkPmlBaseline",
        });
        cy.then(() => {
          const baseline = globalState.get("sdkPmlBaseline");
          // The SDK's own list must agree with the server-integration create:
          // same pinned token and same list contents.
          expect(
            baseline.customer_payment_methods[0].payment_token,
            "SDK list token matches the enriched create's token"
          ).to.equal(globalState.get("basePaymentToken"));
          expect(baseline.customer_payment_methods[0]).to.deep.equal(
            globalState.get("enrichedCreateCpm")
          );
          expect(baseline.payment_methods_enabled).to.deep.equal(
            globalState.get("enrichedCreateEnabled")
          );
        });
      });

      it("Repeated SDK calls agree on one pinned token (issue case 6)", () => {
        // Cypress runs requests off its serialized queue, so these are
        // sequential rather than racing — they guard the read path (every call
        // hits the same pinned state); the SETNX race itself is loadtest
        // territory.
        for (let i = 0; i < 10; i++) {
          cy.clientPaymentMethodsListCall(globalState, {
            compareTo: "sdkPmlBaseline",
          });
        }
      });

      it("Session tokens return the same vault authorization (issue case 3)", () => {
        cy.sessionTokensStabilityCall(globalState);
        cy.sessionTokensStabilityCall(globalState);
      });

      it("Update intent with X-Integration-Type: server agrees with the SDK routes (issue case 4)", () => {
        const updateData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ServerIntegrationUpdateAmount"];

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("apiKey"),
            "X-Integration-Type": "server",
          },
          failOnStatusCode: false,
          body: updateData.Request,
        }).then((response) => {
          expect(response.status, "status_code").to.equal(200);
          expect(response.body.amount, "amount").to.equal(8940);
          expect(response.body.status, "status").to.equal(
            "requires_payment_method"
          );

          const list = response.body.payment_method_list;
          expect(list, "payment_method_list").to.be.an("object").and.to.not.be
            .null;
          // The amount change rebuilt the list (intent modified_at moved), but
          // the token inside it is still the pinned one.
          expect(
            list.customer_payment_methods[0].payment_token,
            "pinned payment token survives the update"
          ).to.equal(globalState.get("basePaymentToken"));
          expect(list.payment_methods_enabled).to.deep.equal(
            globalState.get("enrichedCreateEnabled")
          );

          const stashed = globalState.get("sdkVaultAuthorization");
          const authorization =
            response.body.session_tokens?.vault_details?.vault_data
              ?.sdk_authorization ?? null;
          if (authorization && stashed) {
            expect(
              authorization,
              "vault sdk authorization matches the standalone route's"
            ).to.equal(stashed);
          }
        });
      });

      it("SDK list after the update still returns the pinned token", () => {
        const baseline = globalState.get("sdkPmlBaseline");
        // Full-body byte equality no longer holds (intent_data.amount moved),
        // but the pinned token and the enabled methods are unchanged.
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/client?client_secret=${globalState.get("clientSecret")}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("publishableKey"),
          },
        }).then((response) => {
          expect(response.status, "status_code").to.equal(200);
          expect(
            response.body.customer_payment_methods[0].payment_token,
            "pinned token after rebuilt list"
          ).to.equal(globalState.get("basePaymentToken"));
          expect(response.body.payment_methods_enabled).to.deep.equal(
            baseline.payment_methods_enabled
          );
          expect(response.body.intent_data.amount, "intent amount").to.equal(
            8940
          );
        });
      });

      it("Client-integration update is unaffected (issue case 5)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ServerIntegrationUpdate"];
        cy.updatePaymentIntentServerIntegrationTest(data, globalState, null);
      });

      it("SDK list after the client update still returns the pinned token", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/client?client_secret=${globalState.get("clientSecret")}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("publishableKey"),
          },
        }).then((response) => {
          expect(response.status, "status_code").to.equal(200);
          expect(
            response.body.customer_payment_methods[0].payment_token,
            "pinned token after a client-integration update"
          ).to.equal(globalState.get("basePaymentToken"));
        });
      });
    }
  );

  context(
    "merchant connector config change lands on the next call (issue case 7)",
    () => {
      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Save a card for the customer (setup payment)", () => {
        const saveCardData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          saveCardData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Combined PML lists the saved card (baseline)", () => {
        cy.clientPaymentMethodsListCall(globalState, {
          storeAs: "connToggleBaseline",
        });
        cy.then(() => {
          const baseline = globalState.get("connToggleBaseline");
          expect(baseline.payment_methods_enabled).to.be.an("array").and.to.not
            .be.empty;
          expect(
            baseline.customer_payment_methods,
            "customer_payment_methods has the saved card"
          )
            .to.be.an("array")
            .and.to.have.length(1);
          globalState.set(
            "basePaymentToken",
            baseline.customer_payment_methods[0].payment_token
          );
        });
      });

      it("Disable the connector — next PML call is empty", () => {
        cy.connectorDisabledCallTest(globalState, true);
        cy.clientPaymentMethodsListCall(globalState, {
          expectEmptyEnabled: true,
        });
      });

      it("Re-enable the connector — the same pinned token is restored", () => {
        cy.connectorDisabledCallTest(globalState, false);
        const baseline = globalState.get("connToggleBaseline");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/client?client_secret=${globalState.get("clientSecret")}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("publishableKey"),
          },
        }).then((response) => {
          expect(response.status, "status_code").to.equal(200);
          expect(response.body.payment_methods_enabled).to.deep.equal(
            baseline.payment_methods_enabled
          );
          expect(
            response.body.customer_payment_methods[0].payment_token,
            "pinned token restored after re-enable"
          ).to.equal(globalState.get("basePaymentToken"));
        });
      });
    }
  );
});
