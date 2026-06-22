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

  // Create connector_3, connector_4, connector_5 with hardcoded profile names
  // These profile names (profile2, profile3, profile4) are used by bank debit tests
  [
    { num: 3, profileName: "profile2", mcaName: "merchantConnector2" },
    { num: 4, profileName: "profile3", mcaName: "merchantConnector3" },
    { num: 5, profileName: "profile4", mcaName: "merchantConnector4" },
  ].forEach(({ num, profileName, mcaName }) => {
    context(
      `Create business profile and merchant connector account for connector_${num}`,
      () => {
        it(`Create business profile for connector_${num}`, () => {
          cy.createBusinessProfileTest(
            fixtures.businessProfile.bpCreate,
            globalState,
            profileName
          );
        });

        it(`Create merchant connector account for connector_${num}`, () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState,
            profileName,
            mcaName
          );
        });
      }
    );
  });
});
