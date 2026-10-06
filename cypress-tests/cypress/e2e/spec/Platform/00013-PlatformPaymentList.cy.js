import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let savedState;

describe("Platform Payment List", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // The platform user and merchant, connected merchants, api keys,
      // connectors and customers are created by the Platform setup specs
      // (00001 - 00004); reuse them here instead of creating duplicates
      globalState.set(
        "paymentListUserInfoToken",
        globalState.get("userInfoToken")
      );
      globalState.set(
        "paymentListMerchantId",
        globalState.get("platformMerchantId")
      );
      // Save pre-spec state so it can be restored for any spec running after
      // this one (the standard merchant signup below overwrites these keys)
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
      globalState.set("apiKey", globalState.get("platformApiKey"));
      globalState.set("profileId", globalState.get("profileIdCm1"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 6501 },
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("connectedMerchantId1")
      );

      cy.then(() => {
        globalState.set(
          "paymentListOboPaymentId1",
          globalState.get("paymentID")
        );
      });
    });

    it("platform-creates-obo-payment-for-cm2", () => {
      globalState.set("apiKey", globalState.get("platformApiKey"));
      globalState.set("profileId", globalState.get("profileIdCm2"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 4502 },
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("connectedMerchantId2")
      );

      cy.then(() => {
        globalState.set(
          "paymentListOboPaymentId2",
          globalState.get("paymentID")
        );
      });
    });

    it("cm1-creates-own-payment", () => {
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("profileId", globalState.get("profileIdCm1"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        { ...fixtures.createConfirmPaymentBody, amount: 3203 },
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        globalState.set(
          "paymentListCm1OwnPaymentId",
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
          "paymentListStdMerchantId",
          globalState.get("merchantId")
        );
        globalState.set(
          "paymentListStdUserInfoToken",
          globalState.get("userInfoToken")
        );
      });
    });

    it("list-with-standard-merchant-jwt-returns-401", () => {
      cy.platformPaymentListCallTest(null, globalState, {
        token: globalState.get("paymentListStdUserInfoToken"),
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
      cy.platformPaymentListCallTest({ limit: 100 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedMerchantId: globalState.get("paymentListMerchantId"),
        expectedProcessorMerchantIds: [
          globalState.get("connectedMerchantId1"),
          globalState.get("connectedMerchantId2"),
        ],
        expectedContainsPaymentIds: [
          globalState.get("paymentListOboPaymentId1"),
          globalState.get("paymentListOboPaymentId2"),
          globalState.get("paymentListCm1OwnPaymentId"),
        ],
      });
    });

    it("list-limit-1-returns-single-payment", () => {
      cy.platformPaymentListCallTest({ limit: 1 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 1,
      });
    });
  });

  context("Platform Payment List Filters", () => {
    it("list-payment-id-filter-returns-obo-payment", () => {
      cy.platformPaymentListCallTest(
        { payment_id: globalState.get("paymentListOboPaymentId1") },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("paymentListOboPaymentId1"),
        }
      );
    });

    it("list-payment-id-filter-returns-connected-merchant-own-payment", () => {
      cy.platformPaymentListCallTest(
        { payment_id: globalState.get("paymentListCm1OwnPaymentId") },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("paymentListCm1OwnPaymentId"),
          expectedProcessorMerchantId: globalState.get("connectedMerchantId1"),
        }
      );
    });

    it("list-processor-merchant-id-cm1-returns-cm1-payments", () => {
      cy.platformPaymentListCallTest(
        {
          processor_merchant_id: globalState.get("connectedMerchantId1"),
          limit: 100,
        },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedProcessorMerchantId: globalState.get("connectedMerchantId1"),
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
            globalState.get("paymentListCm1OwnPaymentId"),
          ],
        }
      );
    });

    it("list-processor-merchant-id-other-org-merchant-returns-no-payments", () => {
      cy.platformPaymentListCallTest(
        {
          processor_merchant_id: globalState.get("paymentListStdMerchantId"),
        },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 0,
          expectedTotalCount: 0,
        }
      );
    });

    it("list-amount-range-returns-matching-payment", () => {
      cy.platformPaymentListCallTest(
        { start_amount: 6001, end_amount: 7000 },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedCount: 1,
          expectedTotalCount: 1,
          expectedPaymentId: globalState.get("paymentListOboPaymentId1"),
        }
      );
    });

    it("list-currency-eur-returns-no-payments", () => {
      cy.platformPaymentListCallTest({ currency: "EUR" }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedCount: 0,
        expectedTotalCount: 0,
      });
    });

    it("list-status-succeeded-returns-succeeded-payments", () => {
      cy.platformPaymentListCallTest(
        { status: "succeeded", limit: 100 },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedPaymentStatus: "succeeded",
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
          ],
        }
      );
    });

    it("list-status-failed-returns-no-payments", () => {
      cy.platformPaymentListCallTest({ status: "failed" }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
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
          limit: 100,
        },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
            globalState.get("paymentListOboPaymentId2"),
            globalState.get("paymentListCm1OwnPaymentId"),
          ],
        }
      );
    });

    it("list-end-time-without-start-time-returns-payments", () => {
      cy.platformPaymentListCallTest(
        { end_time: new Date().toISOString(), limit: 100 },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
            globalState.get("paymentListOboPaymentId2"),
            globalState.get("paymentListCm1OwnPaymentId"),
          ],
        }
      );
    });

    it("list-sorted-by-amount-ascending", () => {
      cy.platformPaymentListCallTest(
        { on: "amount", by: "asc", limit: 100 },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectAmountAscending: true,
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
            globalState.get("paymentListOboPaymentId2"),
            globalState.get("paymentListCm1OwnPaymentId"),
          ],
        }
      );
    });

    it("list-sorted-on-amount-without-by-returns-payments", () => {
      cy.platformPaymentListCallTest(
        { on: "amount", limit: 100 },
        globalState,
        {
          token: globalState.get("paymentListUserInfoToken"),
          expectedStatus: 200,
          expectedContainsPaymentIds: [
            globalState.get("paymentListOboPaymentId1"),
            globalState.get("paymentListOboPaymentId2"),
            globalState.get("paymentListCm1OwnPaymentId"),
          ],
        }
      );
    });
  });

  context("Platform Payment List Validation Negatives", () => {
    it("list-limit-0-returns-400", () => {
      cy.platformPaymentListCallTest({ limit: 0 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list limit 0 is invalid, it must be between 1 and 100",
      });
    });

    it("list-limit-101-returns-400", () => {
      cy.platformPaymentListCallTest({ limit: 101 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list limit 101 is invalid, it must be between 1 and 100",
      });
    });

    it("list-offset-20001-returns-400", () => {
      cy.platformPaymentListCallTest({ offset: 20001 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: list offset 20001 is invalid, it must be at most 20000",
      });
    });

    it("list-status-invalid-enum-returns-400", () => {
      cy.platformPaymentListCallTest({ status: "not_a_status" }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 400,
        expectedBodyIncludes:
          "Query deserialize error: Invalid value 'not_a_status': Matching variant not found",
      });
    });
  });

  context("Platform Payment Filter Endpoint", () => {
    it("filter-with-valid-platform-jwt-returns-filter-options", () => {
      cy.platformPaymentFilterCallTest(globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedConnectorName: utils.getOriginalConnectorName(
          globalState.get("connectorId")
        ),
        expectedMerchantConnectorIds: [
          globalState.get("connectorIdCm1"),
          globalState.get("connectorIdCm2"),
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
      cy.platformPaymentListCallTest({ limit: 100 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedMerchantId: globalState.get("paymentListMerchantId"),
        expectedProcessorMerchantIds: [
          globalState.get("connectedMerchantId1"),
          globalState.get("connectedMerchantId2"),
        ],
        expectedContainsPaymentIds: [
          globalState.get("paymentListOboPaymentId1"),
          globalState.get("paymentListOboPaymentId2"),
          globalState.get("paymentListCm1OwnPaymentId"),
        ],
      });
    });

    it("repeat-list-call-returns-same-results-2", () => {
      cy.platformPaymentListCallTest({ limit: 100 }, globalState, {
        token: globalState.get("paymentListUserInfoToken"),
        expectedStatus: 200,
        expectedMerchantId: globalState.get("paymentListMerchantId"),
        expectedProcessorMerchantIds: [
          globalState.get("connectedMerchantId1"),
          globalState.get("connectedMerchantId2"),
        ],
        expectedContainsPaymentIds: [
          globalState.get("paymentListOboPaymentId1"),
          globalState.get("paymentListOboPaymentId2"),
          globalState.get("paymentListCm1OwnPaymentId"),
        ],
      });
    });
  });
});
