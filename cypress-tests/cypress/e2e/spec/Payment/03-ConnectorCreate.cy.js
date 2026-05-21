import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

const bankDebitPaymentMethodsSepa = [
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "sepa",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
];

const bankDebitPaymentMethodsBecs = [
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "becs",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
];

const bankDebitPaymentMethodsBacs = [
  {
    payment_method: "bank_debit",
    payment_method_types: [
      {
        payment_method_type: "bacs",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: true,
        installment_payment_enabled: true,
      },
    ],
  },
];

let globalState;
describe("Connector Account Create flow test", () => {
  let isBankDebitConnector = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      isBankDebitConnector = !shouldIncludeConnector(
        globalState.get("connectorId"),
        CONNECTOR_LISTS.INCLUDE.STRIPE_BANK_DEBIT
      );
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

  // subsequent profile and mca ids should check for the existence of multiple connectors
  context(
    "Create another business profile and merchant connector account if MULTIPLE_CONNECTORS flag is true",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true }
        );
      });

      it("Create merchant connector account", () => {
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

  context(
    "Create business profile and merchant connector account for connector_3 (bank_debit BACS)",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_3", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile2"
        );
      });

      it("Create merchant connector account for connector_3", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankDebitPaymentMethodsBacs,
          globalState,
          "profile2",
          "merchantConnector2"
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_4 (bank_debit BECS)",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_4", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile3"
        );
      });

      it("Create merchant connector account for connector_4", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankDebitPaymentMethodsBecs,
          globalState,
          "profile3",
          "merchantConnector3"
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_5 (bank_debit SEPA)",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_5", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile4"
        );
      });

      it("Create merchant connector account for connector_5", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankDebitPaymentMethodsSepa,
          globalState,
          "profile4",
          "merchantConnector4"
        );
      });
    }
  );
});
