import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;
let savedState;

describe("Platform Payment List", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Save pre-spec state so it can be restored for any spec running after
      // this one (the signup + user login flow below overwrites these keys)
      savedState = {
        merchantId: globalState.get("merchantId"),
        organizationId: globalState.get("organizationId"),
        profileId: globalState.get("profileId"),
        apiKey: globalState.get("apiKey"),
        publishableKey: globalState.get("publishableKey"),
        email: globalState.get("email"),
        password: globalState.get("password"),
        totpToken: globalState.get("totpToken"),
        userInfoToken: globalState.get("userInfoToken"),
        customerId: globalState.get("customerId"),
        merchantConnectorId: globalState.get("merchantConnectorId"),
      };
    });
  });

  after("flush global state", () => {
    // Restore pre-spec state before flushing
    for (const [key, value] of Object.entries(savedState)) {
      globalState.set(key, value);
    }
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup Platform Organization With Connected Merchants", () => {
    it("signup-platform-user", () => {
      cy.signupUserWithMerchant("QAPlatformList", globalState, "platform");
    });

    it("platform-user-signin", () => {
      cy.userLogin(globalState);
    });

    it("platform-user-terminate-2fa", () => {
      cy.terminate2Fa(globalState);
    });

    it("platform-user-info", () => {
      cy.userInfo(globalState);

      cy.then(() => {
        globalState.set(
          "platformListUserInfoToken",
          globalState.get("userInfoToken")
        );
        globalState.set(
          "platformListMerchantId",
          globalState.get("merchantId")
        );
        globalState.set(
          "platformListOrganizationId",
          globalState.get("organizationId")
        );
        globalState.set("platformListProfileId", globalState.get("profileId"));
      });
    });

    it("create-api-key-for-platform-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("merchantId", globalState.get("platformListMerchantId"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("platformListApiKey", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
      });
    });

    it("create-connected-merchant-1", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_name: "Platform List Connected Merchant 1",
        merchant_account_type: "connected",
        organization_id: globalState.get("platformListOrganizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "connected",
        merchantIdStateKey: "platformListCm1",
        profileIdStateKey: "platformListProfileIdCm1",
        publishableKeyStateKey: "platformListPublishableKeyCm1",
      });
    });

    it("create-api-key-for-connected-merchant-1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("platformListCm1"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("platformListApiKeyCm1", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("create-connected-merchant-2", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_name: "Platform List Connected Merchant 2",
        merchant_account_type: "connected",
        organization_id: globalState.get("platformListOrganizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "connected",
        merchantIdStateKey: "platformListCm2",
        profileIdStateKey: "platformListProfileIdCm2",
        publishableKeyStateKey: "platformListPublishableKeyCm2",
      });
    });

    it("create-api-key-for-connected-merchant-2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("platformListCm2"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("platformListApiKeyCm2", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("create-connector-for-connected-merchant-1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");

      globalState.set("merchantId", globalState.get("platformListCm1"));
      globalState.set("apiKey", globalState.get("platformListApiKeyCm1"));
      globalState.set("profileId", globalState.get("platformListProfileIdCm1"));

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "platformListConnectorIdCm1",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
      });
    });

    it("create-connector-for-connected-merchant-2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");

      globalState.set("merchantId", globalState.get("platformListCm2"));
      globalState.set("apiKey", globalState.get("platformListApiKeyCm2"));
      globalState.set("profileId", globalState.get("platformListProfileIdCm2"));

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "platformListConnectorIdCm2",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
      });
    });

    it("create-customer-with-connected-merchant-1-key", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("platformListCm1"));
      globalState.set("apiKey", globalState.get("platformListApiKeyCm1"));

      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      cy.then(() => {
        globalState.set(
          "platformListCustomerId",
          globalState.get("customerId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Seed Payments For Platform List", () => {
    let savedApiKey, savedProfileId, savedCustomerId, savedMerchantConnectorId;

    before(() => {
      savedApiKey = globalState.get("apiKey");
      savedProfileId = globalState.get("profileId");
      savedCustomerId = globalState.get("customerId");
      savedMerchantConnectorId = globalState.get("merchantConnectorId");
    });

    after(() => {
      globalState.set("apiKey", savedApiKey);
      globalState.set("profileId", savedProfileId);
      globalState.set("customerId", savedCustomerId);
      globalState.set("merchantConnectorId", savedMerchantConnectorId);
    });

    it("platform-creates-obo-payment-for-cm1", () => {
      globalState.set("apiKey", globalState.get("platformListApiKey"));
      globalState.set("profileId", globalState.get("platformListProfileIdCm1"));
      globalState.set("customerId", globalState.get("platformListCustomerId"));
      globalState.set(
        "merchantConnectorId",
        globalState.get("platformListConnectorIdCm1")
      );

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 6000 },
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("platformListCm1")
      );

      cy.then(() => {
        globalState.set(
          "platformListOboPaymentId1",
          globalState.get("paymentID")
        );
      });
    });

    it("platform-creates-obo-payment-for-cm2", () => {
      globalState.set("apiKey", globalState.get("platformListApiKey"));
      globalState.set("profileId", globalState.get("platformListProfileIdCm2"));
      globalState.set("customerId", globalState.get("platformListCustomerId"));
      globalState.set(
        "merchantConnectorId",
        globalState.get("platformListConnectorIdCm2")
      );

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 4500 },
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("platformListCm2")
      );

      cy.then(() => {
        globalState.set(
          "platformListOboPaymentId2",
          globalState.get("paymentID")
        );
      });
    });

    it("cm1-creates-own-payment", () => {
      globalState.set("apiKey", globalState.get("platformListApiKeyCm1"));
      globalState.set("profileId", globalState.get("platformListProfileIdCm1"));
      globalState.set("customerId", globalState.get("platformListCustomerId"));
      globalState.set(
        "merchantConnectorId",
        globalState.get("platformListConnectorIdCm1")
      );

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 3200 },
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        globalState.set(
          "platformListCm1OwnPaymentId",
          globalState.get("paymentID")
        );
      });
    });
  });

  context("Platform Payment List Authentication", () => {
    it("list-without-auth-header-returns-400", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        noAuth: true,
        expectedStatus: 400,
        expectedError: {
          type: "invalid_request",
          message: "Missing required param: Authorization",
          code: "IR_04",
        },
      });
    });

    it("list-with-invalid-jwt-returns-401", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: "invalid.platform.list.jwt.token",
        expectedStatus: 401,
        expectedError: {
          type: "invalid_request",
          message: "Access forbidden, invalid JWT token was used",
          code: "IR_17",
        },
      });
    });
  });

  context("Standard Merchant JWT Cannot List Platform Payments", () => {
    let savedEmail,
      savedPassword,
      savedTotpToken,
      savedUserInfoToken,
      savedMerchantId,
      savedOrganizationId,
      savedProfileId;

    before(() => {
      savedEmail = globalState.get("email");
      savedPassword = globalState.get("password");
      savedTotpToken = globalState.get("totpToken");
      savedUserInfoToken = globalState.get("userInfoToken");
      savedMerchantId = globalState.get("merchantId");
      savedOrganizationId = globalState.get("organizationId");
      savedProfileId = globalState.get("profileId");
    });

    after(() => {
      globalState.set("email", savedEmail);
      globalState.set("password", savedPassword);
      globalState.set("totpToken", savedTotpToken);
      globalState.set("userInfoToken", savedUserInfoToken);
      globalState.set("merchantId", savedMerchantId);
      globalState.set("organizationId", savedOrganizationId);
      globalState.set("profileId", savedProfileId);
    });

    it("signup-standard-user", () => {
      cy.signupUserWithMerchant("QAPlatformListStd", globalState);
    });

    it("standard-user-signin", () => {
      cy.userLogin(globalState);
    });

    it("standard-user-terminate-2fa", () => {
      cy.terminate2Fa(globalState);
    });

    it("standard-user-info", () => {
      cy.userInfo(globalState);

      cy.then(() => {
        globalState.set(
          "platformListStdMerchantId",
          globalState.get("merchantId")
        );
        globalState.set(
          "platformListStdUserInfoToken",
          globalState.get("userInfoToken")
        );
      });
    });

    it("list-with-standard-merchant-jwt-returns-401", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: globalState.get("platformListStdUserInfoToken"),
        expectedStatus: 401,
        expectedError: {
          type: "invalid_request",
          message: "API key not provided or invalid API key used",
          code: "IR_01",
        },
      });
    });
  });

  context("Platform Payment List Happy Path", () => {
    it("list-with-valid-platform-jwt-returns-all-org-payments", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 3,
        expectedTotalCount: 3,
        expectedMerchantId: globalState.get("platformListMerchantId"),
        expectedProcessorMerchantIds: [
          globalState.get("platformListCm1"),
          globalState.get("platformListCm2"),
        ],
        expectedPaymentIds: [
          globalState.get("platformListOboPaymentId1"),
          globalState.get("platformListOboPaymentId2"),
          globalState.get("platformListCm1OwnPaymentId"),
        ],
      });
    });

    it("list-limit-1-returns-single-payment", () => {
      cy.platformPaymentListCallTest({ limit: 1 }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 1,
        expectedTotalCount: 3,
      });
    });
  });

  context("Platform Payment List Filters", () => {
    it("list-payment-id-filter-returns-obo-payment", () => {
      cy.platformPaymentListCallTest(
        { payment_id: globalState.get("platformListOboPaymentId1") },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("platformListOboPaymentId1"),
        }
      );
    });

    it("list-payment-id-filter-returns-connected-merchant-own-payment", () => {
      cy.platformPaymentListCallTest(
        { payment_id: globalState.get("platformListCm1OwnPaymentId") },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("platformListCm1OwnPaymentId"),
          expectedProcessorMerchantId: globalState.get("platformListCm1"),
        }
      );
    });

    it("list-processor-merchant-id-cm1-returns-cm1-payments", () => {
      cy.platformPaymentListCallTest(
        { processor_merchant_id: globalState.get("platformListCm1") },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 2,
          expectedTotalCount: 2,
          expectedProcessorMerchantId: globalState.get("platformListCm1"),
        }
      );
    });

    it("list-processor-merchant-id-other-org-merchant-returns-no-payments", () => {
      cy.platformPaymentListCallTest(
        {
          processor_merchant_id: globalState.get("platformListStdMerchantId"),
        },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 0,
          expectedTotalCount: 0,
        }
      );
    });

    it("list-amount-range-returns-matching-payment", () => {
      cy.platformPaymentListCallTest(
        { start_amount: 5000, end_amount: 6000 },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("platformListOboPaymentId1"),
        }
      );
    });

    it("list-currency-eur-returns-no-payments", () => {
      cy.platformPaymentListCallTest({ currency: "EUR" }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 0,
        expectedTotalCount: 0,
      });
    });

    it("list-status-succeeded-returns-all-payments", () => {
      cy.platformPaymentListCallTest({ status: "succeeded" }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 3,
        expectedTotalCount: 3,
      });
    });

    it("list-status-failed-returns-no-payments", () => {
      cy.platformPaymentListCallTest({ status: "failed" }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 0,
        expectedTotalCount: 0,
      });
    });

    it("list-time-range-returns-all-payments", () => {
      cy.platformPaymentListCallTest(
        {
          start_time: "2020-01-01T00:00:00Z",
          end_time: new Date().toISOString(),
        },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 3,
          expectedTotalCount: 3,
        }
      );
    });

    it("list-end-time-without-start-time-returns-payments", () => {
      cy.platformPaymentListCallTest(
        { end_time: new Date().toISOString() },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 3,
          expectedTotalCount: 3,
        }
      );
    });

    it("list-sorted-by-amount-ascending", () => {
      cy.platformPaymentListCallTest(
        { on: "amount", by: "asc", limit: 100 },
        globalState,
        {
          token: globalState.get("platformListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 3,
          expectedTotalCount: 3,
          expectAmountAscending: true,
        }
      );
    });

    it("list-sorted-on-amount-without-by-returns-payments", () => {
      cy.platformPaymentListCallTest({ on: "amount" }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 3,
        expectedTotalCount: 3,
      });
    });
  });

  context("Platform Payment List Validation Negatives", () => {
    it("list-limit-0-returns-400", () => {
      cy.platformPaymentListCallTest({ limit: 0 }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list limit 0 is invalid, it must be between 1 and 100",
      });
    });

    it("list-limit-101-returns-400", () => {
      cy.platformPaymentListCallTest({ limit: 101 }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list limit 101 is invalid, it must be between 1 and 100",
      });
    });

    it("list-offset-20001-returns-400", () => {
      cy.platformPaymentListCallTest({ offset: 20001 }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list offset 20001 is invalid, it must be at most 20000",
      });
    });

    it("list-status-invalid-enum-returns-400", () => {
      cy.platformPaymentListCallTest({ status: "not_a_status" }, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: Invalid value 'not_a_status': Matching variant not found",
      });
    });
  });

  context("Platform Payment Filter Endpoint", () => {
    it("filter-with-valid-platform-jwt-returns-filter-options", () => {
      cy.platformPaymentFilterCallTest(globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedConnectorName: utils.getOriginalConnectorName(
          globalState.get("connectorId")
        ),
        expectedMerchantConnectorIds: [
          globalState.get("platformListConnectorIdCm1"),
          globalState.get("platformListConnectorIdCm2"),
        ],
      });
    });

    it("filter-with-invalid-jwt-returns-401", () => {
      cy.platformPaymentFilterCallTest(globalState, {
        token: "invalid.platform.list.jwt.token",
        expectedStatus: 401,
        expectedError: {
          type: "invalid_request",
          message: "Access forbidden, invalid JWT token was used",
          code: "IR_17",
        },
      });
    });
  });

  context("Platform Payment List Determinism", () => {
    it("repeat-list-call-returns-same-results-1", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 3,
        expectedTotalCount: 3,
        expectedMerchantId: globalState.get("platformListMerchantId"),
        expectedPaymentIds: [
          globalState.get("platformListOboPaymentId1"),
          globalState.get("platformListOboPaymentId2"),
          globalState.get("platformListCm1OwnPaymentId"),
        ],
      });
    });

    it("repeat-list-call-returns-same-results-2", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: globalState.get("platformListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 3,
        expectedTotalCount: 3,
        expectedMerchantId: globalState.get("platformListMerchantId"),
        expectedPaymentIds: [
          globalState.get("platformListOboPaymentId1"),
          globalState.get("platformListOboPaymentId2"),
          globalState.get("platformListCm1OwnPaymentId"),
        ],
      });
    });
  });
});
