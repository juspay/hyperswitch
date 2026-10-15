import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Payment fingerprint is generated regardless of the blocklist guard", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card payment with the blocklist guard disabled (the default) still gets a fingerprint",
    () => {
      it("create-confirm-payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("retrieve-payment-and-check-fingerprint", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status, "status").to.equal(200);
          // The actual digest is nondeterministic (derived from the card PAN and a
          // per-merchant secret), so this only asserts generation happened at all —
          // before PR #14510, both fields stayed null on any profile that didn't
          // have the blocklist guard turned on, which is the default.
          expect(response.body.fingerprint, "fingerprint").to.not.be.null;
          expect(response.body.fingerprint_type, "fingerprint_type").to.equal(
            "fpan"
          );
        });
      });
    }
  );
});
