import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Voucher Payment tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(connector, CONNECTOR_LISTS.INCLUDE.VOUCHER)
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          cy.log(
            `Skipping voucher payment tests for connector: ${connector} -- not in VOUCHER inclusion list`
          );
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Boleto Voucher Payment", () => {
    it("Create and Confirm Boleto Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Boleto Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Boleto"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Boleto"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("OXXO Voucher Payment", () => {
    it("Create and Confirm OXXO Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm OXXO Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Oxxo"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Oxxo"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });

    it("OXXO invalid voucher data format (string instead of null) should error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "voucher_pm"
      ]["OxxoInvalidFormat"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Alfamart Voucher Payment", () => {
    it("Create and Confirm Alfamart Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Alfamart Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Alfamart"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Alfamart"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Indomaret Voucher Payment", () => {
    it("Create and Confirm Indomaret Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Indomaret Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Indomaret"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Indomaret"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Seven-Eleven Voucher Payment", () => {
    it("Create and Confirm Seven-Eleven Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Seven-Eleven Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["SevenEleven"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["SevenEleven"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Lawson Voucher Payment", () => {
    it("Create and Confirm Lawson Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Lawson Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Lawson"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Lawson"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("MiniStop Voucher Payment", () => {
    it("Create and Confirm MiniStop Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm MiniStop Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["MiniStop"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["MiniStop"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("FamilyMart Voucher Payment", () => {
    it("Create and Confirm FamilyMart Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm FamilyMart Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["FamilyMart"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["FamilyMart"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Seicomart Voucher Payment", () => {
    it("Create and Confirm Seicomart Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Seicomart Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Seicomart"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Seicomart"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayEasy Voucher Payment", () => {
    it("Create and Confirm PayEasy Voucher Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm PayEasy Voucher Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PayEasy"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PayEasy"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
