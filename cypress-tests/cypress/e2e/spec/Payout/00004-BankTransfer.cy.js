import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

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
      cy.retrievePayoutCallTest(globalState);
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

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
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
