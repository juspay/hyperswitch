import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

/*
Flow:
- Create Business Profile with connector agnostic feature disabled
- Create Merchant Connector Account and Customer
- Make a Payment
- List Payment Method for Customer using Client Secret (will get PMID)

- Create Business Profile with connector agnostic feature disabled
- Create Merchant Connector Account
- Create Payment Intent
- List Payment Method for Customer -- Empty list; i.e., no payment method should be listed
- Confirm Payment with PMID from previous step (should fail as Connector Mandate ID is not present in the newly created Profile)


- Create Business Profile with connector agnostic feature enabled
- Create Merchant Connector Account and Customer
- Make a Payment
- List Payment Method for Customer using Client Secret (will get PMID)

- Create Business Profile with connector agnostic feature disabled
- Create Merchant Connector Account
- Create Payment Intent
- List Payment Method for Customer -- Empty list; i.e., no payment method should be listed
- Confirm Payment with PMID from previous step (should fail as Connector Mandate ID is not present in the newly created Profile)


- Create Business Profile with connector agnostic feature disabled
- Create Merchant Connector Account and Customer
- Make a Payment
- List Payment Method for Customer using Client Secret (will get PMID)

- Create Business Profile with connector agnostic feature enabled
- Create Merchant Connector Account
- Create Payment Intent
- List Payment Method for Customer using Client Secret (will get PMID which is same as the one from previous step along with Payment Token)
- Confirm Payment with PMID from previous step (should pass as NTID is present in the DB)



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
  before(function () {
    // Changed to regular function instead of arrow function
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip running test against a connector that is added in the exclude list
        if (
          utils.shouldExcludeConnector(
            connector,
            utils.CONNECTOR_LISTS.EXCLUDE.CONNECTOR_AGNOSTIC_NTID
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Connector Agnostic Disabled for both Profile 1 and Profile 2",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
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

      it("Confirm No 3DS MIT (PMID)", () => {
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

      it("Confirm No 3DS MIT (Token)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];
        const commonData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];

        const newData = {
          ...data,
          Response: utils.getConnectorFlowDetails(
            data,
            commonData,
            "ResponseCustom"
          ),
        };
        cy.saveCardConfirmCallTest(
          fixtures.saveCardConfirmBody,
          newData,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Connector Agnostic Enabled for Profile 1 and Disabled for Profile 2",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create business profile", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState
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

      it("Confirm No 3DS MIT (PMID)", () => {
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

      it("Confirm No 3DS MIT (Token)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];
        const commonData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];

        const newData = {
          ...data,
          Response: utils.getConnectorFlowDetails(
            data,
            commonData,
            "ResponseCustom"
          ),
        };
        cy.saveCardConfirmCallTest(
          fixtures.saveCardConfirmBody,
          newData,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Connector Agnostic Disabled for Profile 1 and Enabled for Profile 2",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
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

      it("Confirm No 3DS MIT (PMID)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          7000,
          true /* confirm */,
          "automatic",
          globalState,
          true /* connector_agnostic_mit */
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

      it("Confirm No 3DS MIT (Token)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];

        cy.saveCardConfirmCallTest(
          fixtures.saveCardConfirmBody,
          data,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
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

    it("Confirm No 3DS MIT (PMID)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        7000,
        true,
        "automatic",
        globalState,
        true /* connector_agnostic_mit */
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

    it("Confirm No 3DS MIT (Token)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardConfirmAutoCaptureOffSession"];

      cy.saveCardConfirmCallTest(
        fixtures.saveCardConfirmBody,
        data,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
