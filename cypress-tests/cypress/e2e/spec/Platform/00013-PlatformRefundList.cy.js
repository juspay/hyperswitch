import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import {
  platformRefundsConnectedAccountOperationError,
  platformRefundsInvalidApiKeyError,
  platformRefundsListInvalidOffsetError,
  platformRefundsListLimitAboveMaxError,
  platformRefundsListLimitZeroError,
  refundStatusFilterMap,
} from "../../configs/Payment/Commons";

let globalState;

describe("Platform - Refund List and Filter flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("CM1 Creates Customer for Seed Payments", () => {
    let savedApiKey, savedMerchantId;

    before(() => {
      savedApiKey = globalState.get("apiKey");
      savedMerchantId = globalState.get("merchantId");
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
    });

    after(() => {
      globalState.set("apiKey", savedApiKey);
      globalState.set("merchantId", savedMerchantId);
    });

    it("cm1-creates-customer-for-seed-payments", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });
  });

  context(
    "Platform acts on behalf of Connected Merchant 2 - Create payment and refund for platform refund list",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedProfileId,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedProfileId = globalState.get("profileId");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("platformApiKey"));
        globalState.set(
          "publishableKey",
          globalState.get("platformPublishableKey")
        );
        globalState.set("profileId", globalState.get("profileIdCm2"));
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm2")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create and Confirm Payment -> Partial Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment for CM2 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState,
            globalState.get("connectedMerchantId2")
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment for CM2 using header", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment for CM2");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          const newPartialRefundData = {
            ...partialRefundData,
            Response:
              partialRefundData.ResponseCustom || partialRefundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newPartialRefundData,
            globalState,
            globalState.get("connectedMerchantId2")
          );
        });
      });
    }
  );

  context(
    "Platform acts on behalf of Connected Merchant 1 - Create payment and refund for platform refund list",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedProfileId,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedProfileId = globalState.get("profileId");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("platformApiKey"));
        globalState.set(
          "publishableKey",
          globalState.get("platformPublishableKey")
        );
        globalState.set("profileId", globalState.get("profileIdCm1"));
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          const savedPublishableKey = globalState.get("publishableKey");
          globalState.set(
            "publishableKey",
            globalState.get("publishableKeyCm1")
          );
          cy.paymentMethodsCallTest(globalState).then(() => {
            globalState.set("publishableKey", savedPublishableKey);
          });
        });

        cy.step("Confirm Payment Intent for CM1 using header", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            data,
            true,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];
          const newRefundData = {
            ...refundData,
            Response: refundData.ResponseCustom || refundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );
        });
      });
    }
  );

  context("Platform Refund List", () => {
    let savedApiKey;

    before(() => {
      savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("platformApiKey"));
    });

    after(() => {
      globalState.set("apiKey", savedApiKey);
    });

    it("list-platform-refunds-without-filters-test", () => {
      cy.platformRefundListCallTest(
        {},
        { contains: { refund_id: globalState.get("refundId") } },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-refund-id-test", () => {
      const refundId = globalState.get("refundId");
      cy.platformRefundListCallTest(
        { refund_id: refundId },
        {
          count: 1,
          totalCount: 1,
          match: { refund_id: refundId },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-payment-id-test", () => {
      const paymentId = globalState.get("paymentID");
      cy.platformRefundListCallTest(
        { payment_id: paymentId },
        { match: { payment_id: paymentId } },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-processor-merchant-id-cm1-test", () => {
      const connectedMerchantId1 = globalState.get("connectedMerchantId1");
      cy.platformRefundListCallTest(
        { processor_merchant_id: connectedMerchantId1 },
        {
          match: { processor_merchant_id: connectedMerchantId1 },
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-processor-merchant-id-cm2-test", () => {
      const connectedMerchantId2 = globalState.get("connectedMerchantId2");
      cy.platformRefundListCallTest(
        { processor_merchant_id: connectedMerchantId2 },
        {
          nonEmpty: true,
          match: { processor_merchant_id: connectedMerchantId2 },
          notContains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-profile-id-test", () => {
      const profileIdCm1 = globalState.get("profileIdCm1");
      cy.platformRefundListCallTest(
        { profile_id: profileIdCm1 },
        {
          match: { profile_id: profileIdCm1 },
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-currency-test", () => {
      const currency = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"].Request.currency;
      cy.platformRefundListCallTest(
        { currency: currency },
        {
          match: { currency: currency },
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-connector-test", () => {
      const connector = globalState.get("connectorId");
      cy.platformRefundListCallTest(
        { connector: connector },
        {
          match: { connector: connector },
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-refund-status-test", () => {
      // The refund object status (RefundResponse.status) uses
      // succeeded/failed/pending/review, while the refund_status query
      // param and the list item's refund_status field use the filter
      // variants success/failure/pending/manual_review — map between the
      // two so the filter value is valid for every connector
      const refundStatus =
        refundStatusFilterMap[
          getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "Refund"
          ].Response.body.status
        ];
      cy.platformRefundListCallTest(
        { refund_status: refundStatus },
        {
          match: { refund_status: refundStatus },
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-start-amount-test", () => {
      const refundAmount =
        getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"]
          .Request?.amount ?? fixtures.refundBody.amount;
      cy.platformRefundListCallTest(
        { start_amount: refundAmount },
        {
          minRefundAmount: refundAmount,
          contains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-end-amount-test", () => {
      const refundAmount =
        getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"]
          .Request?.amount ?? fixtures.refundBody.amount;
      cy.platformRefundListCallTest(
        { end_amount: refundAmount - 1 },
        {
          nonEmpty: true,
          maxRefundAmount: refundAmount - 1,
          notContains: { refund_id: globalState.get("refundId") },
        },
        globalState
      );
    });

    it("list-platform-refunds-with-empty-amount-range-test", () => {
      const refundAmount =
        getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"]
          .Request?.amount ?? fixtures.refundBody.amount;
      cy.platformRefundListCallTest(
        { start_amount: refundAmount + 1, end_amount: refundAmount + 999 },
        { empty: true },
        globalState
      );
    });

    it("list-platform-refunds-filtered-by-creation-time-range-test", () => {
      const startTime = new Date(Date.now() - 60 * 60 * 1000).toISOString();
      const endTime = new Date(Date.now() + 60 * 60 * 1000).toISOString();
      cy.platformRefundListCallTest(
        { start_time: startTime, end_time: endTime },
        { contains: { refund_id: globalState.get("refundId") } },
        globalState
      );
    });

    it("list-platform-refunds-with-future-time-range-test", () => {
      const startTime = new Date(Date.now() + 60 * 60 * 1000).toISOString();
      const endTime = new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString();
      cy.platformRefundListCallTest(
        { start_time: startTime, end_time: endTime },
        { empty: true },
        globalState
      );
    });

    it("list-platform-refunds-with-end-time-only-test", () => {
      // end_time without start_time is silently ignored — no validation
      // error is raised and the unfiltered list is returned
      const endTime = new Date(Date.now() + 60 * 60 * 1000).toISOString();
      cy.platformRefundListCallTest(
        { end_time: endTime },
        { contains: { refund_id: globalState.get("refundId") } },
        globalState
      );
    });

    it("list-platform-refunds-with-unknown-payment-id-test", () => {
      cy.platformRefundListCallTest(
        { payment_id: "pay_unknown_payment_id_platform_refund_list" },
        { empty: true },
        globalState
      );
    });

    it("list-platform-refunds-with-limit-test", () => {
      cy.platformRefundListCallTest(
        { limit: 1 },
        { count: 1, contains: { refund_id: globalState.get("refundId") } },
        globalState
      );
    });

    it("list-platform-refunds-with-limit-and-offset-test", () => {
      cy.platformRefundListCallTest(
        { limit: 1, offset: 1 },
        { count: 1, notContains: { refund_id: globalState.get("refundId") } },
        globalState
      );
    });

    it("list-platform-refunds-with-invalid-limit-zero-test", () => {
      cy.platformRefundListCallTest(
        { limit: 0 },
        platformRefundsListLimitZeroError,
        globalState
      );
    });

    it("list-platform-refunds-with-invalid-limit-above-max-test", () => {
      cy.platformRefundListCallTest(
        { limit: 1000 },
        platformRefundsListLimitAboveMaxError,
        globalState
      );
    });

    it("list-platform-refunds-with-invalid-offset-test", () => {
      cy.platformRefundListCallTest(
        { offset: 999999 },
        platformRefundsListInvalidOffsetError,
        globalState
      );
    });
  });

  context("Platform Refund Filters", () => {
    let savedApiKey;

    before(() => {
      savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("platformApiKey"));
    });

    after(() => {
      globalState.set("apiKey", savedApiKey);
    });

    it("retrieve-platform-refund-filters-test", () => {
      cy.platformRefundFilterCallTest({}, globalState);
    });
  });

  context("Platform Refund List and Filter Authorization", () => {
    it("connected-merchant-key-cannot-list-platform-refunds-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      cy.platformRefundListCallTest(
        {},
        platformRefundsConnectedAccountOperationError,
        globalState
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("platform-key-with-connected-merchant-header-cannot-list-platform-refunds-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("platformApiKey"));

      cy.platformRefundListCallTest(
        {},
        platformRefundsConnectedAccountOperationError,
        globalState,
        globalState.get("connectedMerchantId1")
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("standard-merchant-key-cannot-list-platform-refunds-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.platformRefundListCallTest(
        {},
        platformRefundsInvalidApiKeyError,
        globalState
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("missing-api-key-cannot-list-platform-refunds-test", () => {
      cy.platformRefundListCallTest(
        {},
        { ...platformRefundsInvalidApiKeyError, omitApiKey: true },
        globalState
      );
    });

    it("connected-merchant-key-cannot-retrieve-platform-refund-filters-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      cy.platformRefundFilterCallTest(
        platformRefundsConnectedAccountOperationError,
        globalState
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("platform-key-with-connected-merchant-header-cannot-retrieve-platform-refund-filters-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("platformApiKey"));

      cy.platformRefundFilterCallTest(
        platformRefundsConnectedAccountOperationError,
        globalState,
        globalState.get("connectedMerchantId1")
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("standard-merchant-key-cannot-retrieve-platform-refund-filters-test", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.platformRefundFilterCallTest(
        platformRefundsInvalidApiKeyError,
        globalState
      );

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });
  });
});
