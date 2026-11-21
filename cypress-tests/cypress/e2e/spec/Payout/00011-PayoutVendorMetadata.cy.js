import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

// Stripe payouts parse payout metadata into PayoutVendorAccountDetails.
// Metadata that carries vendor details but fails to parse must return the
// parse error naming the field, and metadata without vendor details must keep
// the previous behavior (PR #14846).
describe("[Payout] [Bank Transfer - ACH] Vendor Metadata", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.PAYOUT_VENDOR_METADATA.includes(
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
    payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
  });

  context("[Payout] Vendor Metadata - Valid", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-with-us-phone-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["VendorMetadataCustomer"];

      cy.createCustomerCallTest(
        {
          ...Cypress._.cloneDeep(fixtures.customerCreateBody),
          ...data.Request,
        },
        globalState
      );
    });

    it("create-payout-with-valid-vendor-metadata-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["VendorMetadataValid"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-after-account-attach-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["RetrieveAfterAccountAttach"];

      // Stripe account, transfer and payout calls run in the background
      // attach-account task, scheduled 15s after create plus scheduler lag.
      cy.wait(50000);
      cy.retrievePayoutCallTest(globalState, data);
    });
  });

  context("[Payout] Vendor Metadata - Parse Errors And Absent Metadata", () => {
    [
      { key: "VendorMetadataMissingIndividual", name: "missing-individual" },
      { key: "VendorMetadataMissingVendor", name: "missing-vendor" },
      { key: "VendorMetadataInvalidFieldType", name: "invalid-field-type" },
      { key: "VendorMetadataUnrelatedKeys", name: "unrelated-metadata" },
      { key: "VendorMetadataAbsent", name: "without-metadata" },
      { key: "VendorMetadataNull", name: "null-metadata" },
    ].forEach(({ key, name }) => {
      it(`create-payout-with-${name}-test`, () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["ach_bank_transfer"][key];

        cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
      });
    });
  });
});
