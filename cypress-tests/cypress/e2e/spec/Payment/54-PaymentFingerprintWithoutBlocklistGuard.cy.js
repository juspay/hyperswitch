import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

// PR #14510's fix lives in the `Domain::populate_payment_fingerprint` override on the
// `PaymentConfirm` operation, which only runs through the separate POST
// /payments/{id}/confirm step (`cy.confirmCallTest`) — not through a combined
// create-and-confirm POST /payments call, which uses the `PaymentCreate` operation and
// never reaches that override. Using create-then-confirm here is required for this test
// to actually exercise the fix, not incidental styling.
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
      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("confirm-payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
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
