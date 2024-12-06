import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { payment_methods_enabled } from "../PaymentUtils/Commons";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

/*
Flow:
- Create Business Profile with connector agnostic feature disabled
- Create Merchant Connector Account and Customer
- Make a Payment
- List Payment Method for Customer using Client Secret (will get PMID)

- Create Business Profile with connector agnostic feature enabled
- Create Merchant Connector Account
- Create Payment Intent
- List Payment Method for Customer -- Empty list; i.e., no payment method should be listed
- Confirm Payment with PMID from previous step (should fail as Connector Mandate ID is not present in the newly created Profile)


- Create Business Profile with connector agnostic feature enabled
- Create Merchant Connector Account and Customer
- Make a Payment
- List Payment Method for Customer using Client Secret (will get PMID)

- Create Business Profile with connector agnostic feature enabled
- Create Merchant Connector Account
- Create Payment Intent
- List Payment Method for Customer using Client Secret (will get PMID which is same as the one from previous step along with Payment Token)
- Confirm Payment with PMID from previous step (should pass as NTID is present in the DB)
*/

describe("Connector Agnostic Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
  context(
    "Connector Agnostic Disabled for Profile 1 and Enabled for Profile 2",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      // it("Create Business Profile and Merchant connector account", () => {
      //   utils.createProfileAndConnector(
      //     fixtures,
      //     globalState,
      //     payment_methods_enabled
      //   );
      // });

      it("Create business profile", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("Create merchant connector account", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("List Payment Method for Customer using Client Secret", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });

      // it("Create Business Profile and Merchant connector account", () => {
      //   utils.createProfileAndConnector(
      //     fixtures,
      //     globalState,
      //     payment_methods_enabled,
      //     {
      //       flag: true,
      //       is_connector_agnostic_enabled: true,
      //       collect_billing_address_from_wallet_connector: false,
      //       collect_shipping_address_from_wallet_connector: false,
      //       always_collect_billing_address_from_wallet_connector: false,
      //       always_collect_shipping_address_from_wallet_connector: false,
      //     }
      //   );
      // });

      it("Create business profile", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });
      it("Create merchant connector account", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled
        );
      });

      it("Enable Connector Agnostic for Business Profile", () => {
        utils.updateBusinessProfile(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("List Payment Method for Customer", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];
        const commonData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["MITAutoCapture"];

        const newData = {
          ...data,
          Response: utils.getConnectorFlowDetails(
            data,
            commonData,
            "ResponseCustom"
          ),
        };

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          newData,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context("Connector Agnostic Enabled for Profile 1 and Profile 2", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Business Profile and Merchant connector account", () => {
      utils.createProfileAndConnector(
        fixtures,
        globalState,
        payment_methods_enabled,
        {
          flag: true,
          is_connector_agnostic_enabled: true,
          collect_billing_address_from_wallet_connector: false,
          collect_shipping_address_from_wallet_connector: false,
          always_collect_billing_address_from_wallet_connector: false,
          always_collect_shipping_address_from_wallet_connector: false,
        }
      );
    });

    it("Create business profile", () => {
      utils.createBusinessProfile(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });
    it("Create merchant connector account", () => {
      utils.createMerchantConnectorAccount(
        "payment_processor",
        fixtures.createConnectorBody,
        globalState,
        payment_methods_enabled
      );
    });

    it("Create Customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Enable Connector Agnostic for Business Profile", () => {
      utils.updateBusinessProfile(
        fixtures.businessProfile.bpUpdate,
        true, // is_connector_agnostic_enabled
        false, // collect_billing_address_from_wallet_connector
        false, // collect_shipping_address_from_wallet_connector
        false, // always_collect_billing_address_from_wallet_connector
        false, // always_collect_shipping_address_from_wallet_connector
        globalState
      );
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer using Client Secret", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });

    it("Create Business Profile and Merchant connector account", () => {
      utils.createProfileAndConnector(
        fixtures,
        globalState,
        payment_methods_enabled,
        {
          flag: true,
          is_connector_agnostic_enabled: true,
          collect_billing_address_from_wallet_connector: false,
          collect_shipping_address_from_wallet_connector: false,
          always_collect_billing_address_from_wallet_connector: false,
          always_collect_shipping_address_from_wallet_connector: false,
        }
      );
    });

    it("Create business profile", () => {
      utils.createBusinessProfile(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("Create merchant connector account", () => {
      utils.createMerchantConnectorAccount(
        "payment_processor",
        fixtures.createConnectorBody,
        globalState,
        payment_methods_enabled
      );
    });

    it("Enable Connector Agnostic for Business Profile", () => {
      utils.updateBusinessProfile(
        fixtures.businessProfile.bpUpdate,
        true, // is_connector_agnostic_enabled
        false, // collect_billing_address_from_wallet_connector
        false, // collect_shipping_address_from_wallet_connector
        false, // always_collect_billing_address_from_wallet_connector
        false, // always_collect_shipping_address_from_wallet_connector
        globalState
      );
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });

    it("Confirm No 3DS MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        7000,
        true,
        "automatic",
        globalState
      );
    });
  });
});
