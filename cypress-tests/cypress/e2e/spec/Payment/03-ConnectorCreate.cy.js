import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Connector Account Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Create merchant connector account", () => {
    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  // Create connector_2 (standard multi-connector setup using Utils)
  context(
    "Create business profile and merchant connector account for connector_2",
    () => {
      it("Create business profile for connector_2", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true }
        );
      });

      it("Create merchant connector account for connector_2", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled,
          { nextConnector: true }
        );
      });
    }
  );

  // connector_3/4/5 are only needed for Stripe bank debit multi-credential setup
  // (SEPA=connector_5, BACS=connector_3, BECS=connector_4)
  ["connector_3", "connector_4", "connector_5"].forEach((connectorValue) => {
    context(
      `Create business profile and merchant connector account for ${connectorValue}`,
      () => {
        before(function () {
          const connectorId = globalState.get("connectorId");
          if (!["stripe", "stripeconnect"].includes(connectorId)) {
            this.skip();
          }
        });

        it(`Create business profile for ${connectorValue}`, () => {
          utils.createBusinessProfile(
            fixtures.businessProfile.bpCreate,
            globalState,
            { nextConnector: true, value: connectorValue }
          );
        });

        it(`Create merchant connector account for ${connectorValue}`, () => {
          utils.createMerchantConnectorAccount(
            "payment_processor",
            fixtures.createConnectorBody,
            globalState,
            payment_methods_enabled,
            { nextConnector: true, value: connectorValue }
          );
        });
      }
    );
  });
});
