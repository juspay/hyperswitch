import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";
import getPaymentConnectorDetails from "../../configs/Payment/Utils";

let globalState;

// TODO: Add test for Bank Transfer - ACH
describe.skip("[Payout] [Bank Transfer - ACH]", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });
});

// TODO: Add test for Bank Transfer - BACS
describe.skip("[Payout] [Bank Transfer - BACS]", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });
});

describe("[Payout] [Bank Transfer - SEPA]", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_SEPA.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("[Payout] [Bank transfer - SEPA] Auto Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["Fulfill"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("[Payout] [Bank transfer - SEPA] Manual Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("confirm-payout-call-with-manual-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["Confirm"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("fulfill-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["Fulfill"];

      cy.fulfillPayoutCallTest({}, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});

describe("[Payout] [Bank Transfer - Open Banking]", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("[Payout] [Bank transfer - Open Banking] UCS config setup", () => {
    it("create-ucs-configs-call-test", function () {
      if (globalState.get("connectorId") !== "trustly") {
        this.skip();
      }

      if (!globalState.get("ucsEnabled")) {
        cy.task(
          "cli_log",
          "create-ucs-configs-call-test: UCS_ENABLED env not set - auto-enabling UCS for trustly (UCS-only connector)"
        );
        globalState.set("ucsEnabled", true);
      }

      cy.setupConfigs(globalState, "ucs_enabled", "true");

      cy.createRolloutConfig(globalState, "Webhooks", {
        rollout_percent: 1.0,
        execution_mode: "primary",
        webhook_flows: ["Payout"],
      });
    });
  });

  context("[Payout] [Bank transfer - Open Banking] Auto Fulfill", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["open_banking"]["Fulfill"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["open_banking"]["RetrieveAfterFulfill"];

      if (data) {
        cy.pollPayoutStatusCallTest(globalState);
      }
      cy.retrievePayoutCallTest(globalState, data);
    });
  });

  context("[Payout] [Bank transfer - Open Banking] Manual Fulfill", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("confirm-payout-call-with-manual-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["open_banking"]["Confirm"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    if (
      utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_MANUAL_FULFILL.includes(
        Cypress.env("CONNECTOR")
      )
    ) {
      it("fulfill-payout-call-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["open_banking"]["Fulfill"];

        cy.fulfillPayoutCallTest({}, data, globalState);
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }

    it("retrieve-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["open_banking"]["RetrieveAfterFulfill"];

      if (data) {
        cy.pollPayoutStatusCallTest(globalState);
      }
      cy.retrievePayoutCallTest(globalState, data);
    });
  });

  context(
    "[Payout] [Bank transfer - Open Banking] Create without confirm",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payout-without-confirm-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["open_banking"]["Create"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          data,
          false,
          false,
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );

  if (
    utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_INVALID_ACCOUNT_NUMBER.includes(
      Cypress.env("CONNECTOR")
    )
  ) {
    context(
      "[Payout] [Bank transfer - Open Banking] Invalid Account Number",
      () => {
        const shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("create-payout-with-invalid-account-number-test", () => {
          const data = utils.getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["open_banking"]["InvalidAccountNumber"];

          cy.createConfirmPayoutTest(
            Cypress._.cloneDeep(fixtures.createPayoutBody),
            data,
            true,
            true,
            globalState
          );
        });
      }
    );
  }

  if (
    utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_NO_BILLING.includes(
      Cypress.env("CONNECTOR")
    )
  ) {
    context(
      "[Payout] [Bank transfer - Open Banking] Without Billing Address",
      () => {
        let shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("create customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        it("confirm-payout-call-without-billing-test", () => {
          const data = utils.getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["open_banking"]["ConfirmWithoutBilling"];

          cy.createConfirmPayoutTest(
            Cypress._.cloneDeep(fixtures.createPayoutBody),
            data,
            true,
            true,
            globalState
          );
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payout-call-test", () => {
          const data = utils.getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["open_banking"]["RetrieveAfterFulfill"];

          cy.pollPayoutStatusCallTest(globalState);
          cy.retrievePayoutCallTest(globalState, data);
        });
      }
    );
  }

  if (
    utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_INVALID_REFERENCE_FULFILL.includes(
      Cypress.env("CONNECTOR")
    )
  ) {
    context(
      "[Payout] [Bank transfer - Open Banking] Invalid Billing Descriptor",
      () => {
        const shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("create-payout-with-invalid-billing-descriptor-test", () => {
          const data = utils.getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["open_banking"][
            "InvalidBillingDescriptorConfirm"
          ];

          cy.createConfirmPayoutTest(
            fixtures.createPayoutBody,
            data,
            true,
            false,
            globalState
          );
        });
      }
    );
  }

  // Closed-loop payout (PR #14535): the passthrough tokens used below are
  // taken from a live truelayer payment - sender_payment_instrument_id and
  // connector_customer_id of the payment response become the payout's
  // psp_token and psp_customer_id - and are sent with token_type
  // `open_banking` on the bank payout rail, which PR #14535 maps to the UCS
  // OPEN_BANKING payout method while forwarding billing_descriptor.reference
  // and statement_descriptor to UCS.
  if (
    utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_PASSTHROUGH.includes(
      Cypress.env("CONNECTOR")
    )
  ) {
    context(
      "[Payout] [Bank transfer - Open Banking] Closed-loop passthrough (UCS)",
      () => {
        let shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        // sender_payment_instrument_id / connector_customer_id are returned
        // only by UCS-routed truelayer payments, so UCS must be enabled
        // before the closed-loop payment is created (same pattern as the
        // trustly open banking setup above).
        it("create-ucs-configs-call-test", function () {
          if (!globalState.get("ucsEnabled")) {
            cy.task(
              "cli_log",
              "create-ucs-configs-call-test: UCS_ENABLED env not set - auto-enabling UCS for truelayer (UCS-only connector)"
            );
            globalState.set("ucsEnabled", true);
          }

          cy.setupConfigs(globalState, "ucs_enabled", "true");

          const proxyHttp = globalState.get("proxyHttp");
          const proxyHttps = globalState.get("proxyHttps");

          // Authorize covers the confirm call; PSync covers the force-sync
          // retrieve - without its rollout key the router falls back to the
          // legacy interface, which truelayer (UCS-only) does not implement.
          cy.createRolloutConfig(
            globalState,
            "bank_redirect_open_banking_Authorize,bank_redirect_open_banking_PSync",
            {
              rollout_percent: 1.0,
              execution_mode: "primary",
              ...(proxyHttp && proxyHttps
                ? { http_url: proxyHttp, https_url: proxyHttps }
                : {}),
            }
          );

          cy.createRolloutConfig(globalState, "Webhooks", {
            rollout_percent: 1.0,
            execution_mode: "primary",
            webhook_flows: ["Payout"],
          });
        });

        // The payout specs onboard only a payout_processor MCA (00002); the
        // closed-loop source payment additionally needs a payment_processor
        // MCA for bank_redirect/open_banking, otherwise payment routing finds
        // no eligible connector (IR_39). This mirrors the merchant setup used
        // in hyperswitch-cloud#23972. A distinct mcaPrefix keeps globalState
        // "merchantConnectorId" pointing at the payout MCA.
        it("create-truelayer-payment-connector-call-test", () => {
          const paymentConnectorBody = Cypress._.cloneDeep(
            fixtures.createConnectorBody
          );
          // The payout connector create (00002) shares this fixture and flips
          // test_mode to false for truelayer; the payment MCA mirrors the
          // payment-suite setup instead.
          paymentConnectorBody.test_mode = true;

          cy.createConnectorCallTest(
            "payment_processor",
            paymentConnectorBody,
            [
              {
                payment_method: "bank_redirect",
                payment_method_types: [
                  {
                    payment_method_type: "open_banking",
                    payment_experience: null,
                    card_networks: null,
                    accepted_currencies: null,
                    accepted_countries: null,
                    minimum_amount: 1,
                    maximum_amount: 68607706,
                    recurring_enabled: true,
                    installment_payment_enabled: true,
                  },
                ],
              },
            ],
            globalState,
            "profile",
            "paymentProcessorMerchantConnector"
          );
        });

        it("create customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        // Source payment of the closed loop. setup_future_usage=off_session
        // mirrors the merchant flow documented against PR #14535: the same
        // customer's stored bank authorisation is later referenced by the
        // payout through the passthrough tokens.
        it("create-and-confirm-truelayer-payment-test", () => {
          const intentData = {
            Request: {
              currency: "GBP",
              setup_future_usage: "off_session",
            },
            Response: {
              status: 200,
              body: {
                status: "requires_payment_method",
              },
            },
          };
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            intentData,
            "three_ds",
            "automatic",
            globalState
          );

          const confirmData = getPaymentConnectorDetails(
            globalState.get("connectorId")
          )["bank_redirect_pm"]["Truelayer"];
          cy.confirmBankRedirectCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
        });

        it("handle-bank-redirect-redirection-test", () => {
          const expected_redirection = fixtures.confirmBody["return_url"];
          const payment_method_type = globalState.get("paymentMethodType");
          cy.handleBankRedirectRedirection(
            globalState,
            payment_method_type,
            expected_redirection
          );
        });

        it("retrieve-payment-and-save-payout-source-details-test", () => {
          cy.retrieveClosedLoopPayoutSourceCallTest(globalState);
        });

        // cloneDeep both the shared payout fixture and the connector config:
        // billing_descriptor must not leak into the no-billing-descriptor
        // case below (or other specs), and the live passthrough tokens must
        // not pollute the shared config object.
        it("confirm-payout-call-with-auto-fulfill-test", () => {
          const pspToken = globalState.get("senderPaymentInstrumentId");
          const pspCustomerId = globalState.get("connectorCustomerId");
          // Fail fast with a clear message when the payment leg did not
          // produce the closed-loop source details, instead of sending a
          // payout create with missing passthrough tokens.
          expect(
            pspToken,
            "senderPaymentInstrumentId must be set by retrieve-payment-and-save-payout-source-details-test"
          ).to.be.a("string").and.to.not.be.empty;
          expect(
            pspCustomerId,
            "connectorCustomerId must be set by retrieve-payment-and-save-payout-source-details-test"
          ).to.be.a("string").and.to.not.be.empty;

          const data = Cypress._.cloneDeep(
            utils.getConnectorDetails(globalState.get("connectorId"))[
              "bank_transfer_pm"
            ]["open_banking"]["PassthroughBankPayout"]
          );
          data.Request.payout_method_data.passthrough.psp_token = pspToken;
          data.Request.payout_method_data.passthrough.psp_customer_id =
            pspCustomerId;

          cy.createConfirmPayoutTest(
            Cypress._.cloneDeep(fixtures.createPayoutBody),
            data,
            true,
            true,
            globalState
          );
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payout-call-test", () => {
          cy.retrievePayoutCallTest(globalState);
        });

        // truelayer (via UCS) rejects payouts without
        // billing_descriptor.reference - the exact error PR #14535 surfaced
        // in hyperswitch-cloud#23972.
        it("confirm-payout-without-billing-descriptor-test", () => {
          const pspToken = globalState.get("senderPaymentInstrumentId");
          const pspCustomerId = globalState.get("connectorCustomerId");
          expect(
            pspToken,
            "senderPaymentInstrumentId must be set by retrieve-payment-and-save-payout-source-details-test"
          ).to.be.a("string").and.to.not.be.empty;
          expect(
            pspCustomerId,
            "connectorCustomerId must be set by retrieve-payment-and-save-payout-source-details-test"
          ).to.be.a("string").and.to.not.be.empty;

          const data = Cypress._.cloneDeep(
            utils.getConnectorDetails(globalState.get("connectorId"))[
              "bank_transfer_pm"
            ]["open_banking"]["PassthroughBankPayoutWithoutBillingDescriptor"]
          );
          data.Request.payout_method_data.passthrough.psp_token = pspToken;
          data.Request.payout_method_data.passthrough.psp_customer_id =
            pspCustomerId;

          cy.createConfirmPayoutTest(
            Cypress._.cloneDeep(fixtures.createPayoutBody),
            data,
            true,
            true,
            globalState
          );
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });
      }
    );
  }
});

// Payshap / Payshap Proxy fulfill coverage asserts the deterministic
// shipped-environment behavior: the OSS dummy connector base_url ("dev") is
// not a valid URL, so the fulfill call returns 500 HE_00 and the payout
// silently stays in requires_fulfillment.
describe("[Payout] [Bank Transfer - Payshap]", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_PAYSHAP.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("[Payout] [Bank transfer - Payshap] Create without confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-without-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["payshap"]["Create"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        false,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("[Payout] [Bank transfer - Payshap] Create and Confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("confirm-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["payshap"]["Confirm"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context(
    "[Payout] [Bank transfer - Payshap] Create, Confirm and Fulfill",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      // Create, confirm and fulfill intentionally share one it block: fulfill
      // must always target the payout created by the immediately preceding
      // create call, because a fulfill retry on the same payout fails in the
      // process tracker (DuplicateValue) before reaching the connector.
      it("create-confirm-and-fulfill-payout-test", () => {
        const confirmData = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["payshap"]["Confirm"];
        const fulfillData = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["payshap"]["Fulfill"];

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
          return;
        }

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          confirmData,
          true,
          false,
          globalState
        );
        cy.fulfillPayoutCallTest({}, fulfillData, globalState);
        // No should_continue_further on the fulfill data: its expected response
        // is an error body, and the retrieve below must always run to verify
        // the silent-failure state of the payout.
      });

      it("retrieve-payout-after-fulfill-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["payshap"]["RetrieveAfterFulfill"];

        cy.retrievePayoutCallTest(globalState, data);
      });
    }
  );
});

// Payshap Proxy fulfill coverage — see the note above the Payshap describe.
describe("[Payout] [Bank Transfer - Payshap Proxy]", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_PAYSHAP_PROXY.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context(
    "[Payout] [Bank transfer - Payshap Proxy] Create without confirm",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payout-without-confirm-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["payshap_proxy"]["Create"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          data,
          false,
          false,
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );

  context("[Payout] [Bank transfer - Payshap Proxy] Create and Confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("confirm-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["payshap_proxy"]["Confirm"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context(
    "[Payout] [Bank transfer - Payshap Proxy] Create, Confirm and Fulfill",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      // Create, confirm and fulfill intentionally share one it block: fulfill
      // must always target the payout created by the immediately preceding
      // create call, because a fulfill retry on the same payout fails in the
      // process tracker (DuplicateValue) before reaching the connector.
      it("create-confirm-and-fulfill-payout-test", () => {
        const confirmData = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["payshap_proxy"]["Confirm"];
        const fulfillData = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["payshap_proxy"]["Fulfill"];

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
          return;
        }

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          confirmData,
          true,
          false,
          globalState
        );
        cy.fulfillPayoutCallTest({}, fulfillData, globalState);
        // No should_continue_further on the fulfill data: its expected response
        // is an error body, and the retrieve below must always run to verify
        // the silent-failure state of the payout.
      });

      it("retrieve-payout-after-fulfill-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["payshap_proxy"]["RetrieveAfterFulfill"];

        cy.retrievePayoutCallTest(globalState, data);
      });
    }
  );
});
