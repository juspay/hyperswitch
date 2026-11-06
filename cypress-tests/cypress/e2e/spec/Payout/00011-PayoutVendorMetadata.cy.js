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

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("[Payout] Vendor Metadata - Parse Errors", () => {
    [
      {
        key: "VendorMetadataMissingIndividualDetails",
        name: "missing-individual-details",
      },
      {
        key: "VendorMetadataMissingVendorDetails",
        name: "missing-vendor-details",
      },
      { key: "VendorMetadataInvalidFieldType", name: "invalid-field-type" },
    ].forEach(({ key, name }) => {
      it(`create-payout-with-${name}-test`, () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["ach_bank_transfer"][key];

        cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
      });
    });
  });

  context("[Payout] Vendor Metadata - Treated As Absent", () => {
    it("create-payout-with-unrelated-metadata-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["VendorMetadataUnrelatedKeys"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
    });

    it("create-payout-without-metadata-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["VendorMetadataAbsent"];

      delete payoutBody.metadata;

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
    });

    it("create-payout-with-null-metadata-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ach_bank_transfer"]["VendorMetadataNull"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
    });
  });
});
