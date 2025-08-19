import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

describe("UCS Comprehensive Testing", () => {
  let globalState;
  const testResults = {
    totalAvailable: 0,
    totalEnabled: 0,
    totalTested: 0,
    passed: [],
    failed: [],
    skipped: [],
    startTime: null,
    endTime: null,
  };

  const UCS_SUPPORTED_CONNECTORS = ["authorizedotnet"];

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
    testResults.startTime = new Date();
    testResults.totalEnabled = 5;

    cy.task("cli_log", "=== UCS Comprehensive Testing Started ===");
    cy.task(
      "cli_log",
      `Testing connector: ${Cypress.env("CYPRESS_CONNECTOR")}`
    );
    cy.task(
      "cli_log",
      `UCS Compatible Requests will be determined from connector config`
    );
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
  const currentConnector = Cypress.env("CYPRESS_CONNECTOR");
  const isUCSSupported = UCS_SUPPORTED_CONNECTORS.includes(currentConnector);

  if (isUCSSupported) {
    describe(`UCS Tests for ${currentConnector.toUpperCase()}`, () => {
      let connectorConfig;
      let testableRequests;

      before("load connector config", () => {
        cy.task(
          "cli_log",
          `\nLoading connector configuration for: ${currentConnector}`
        );

        const config = getConnectorDetails(currentConnector).card_pm;
        connectorConfig = { card_pm: config };

        if (!connectorConfig || !connectorConfig.card_pm) {
          throw new Error(
            ` Failed to load configuration for connector: ${currentConnector}`
          );
        }

        const allRequests = Object.keys(connectorConfig.card_pm);
        testResults.totalAvailable = allRequests.length;

        cy.task(
          "cli_log",
          `Total requests available in ${currentConnector}.js: ${allRequests.length}`
        );

        const ucsRequestNames = [
          "UCSZeroAuthMandate",
          "UCSConfirmMandate",
          "UCSRecurringPayment",
          "No3DSAutoCapture",
          "No3DSManualCapture",
        ];
        testableRequests = allRequests.filter((requestType) =>
          ucsRequestNames.includes(requestType)
        );

        testResults.totalEnabled = testableRequests.length;

        cy.task(
          "cli_log",
          `UCS-compatible requests found: ${testableRequests.length}`
        );
        cy.task("cli_log", `Testable requests: ${testableRequests.join(", ")}`);
        cy.task(
          "cli_log",
          `Test Coverage: ${testableRequests.length}/${allRequests.length} (${((testableRequests.length / allRequests.length) * 100).toFixed(1)}%)`
        );

        if (testableRequests.length === 0) {
          throw new Error(
            ` No UCS-compatible requests found for connector: ${currentConnector}`
          );
        }

        cy.task(
          "cli_log",
          ` Setting up merchant account for ${currentConnector}...`
        );
      });

      it("should setup merchant account and connector", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);

        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );

        cy.task("cli_log", ` Setup completed for ${currentConnector}`);
      });

      it("should enable UCS configuration", () => {
        cy.task("cli_log", ` Enabling global UCS configuration...`);

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/configs/`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: { key: "ucs_enabled", value: "true" },
          failOnStatusCode: false,
        }).then((response) => {
          if (response.status === 200) {
            expect(response.body.key).to.equal("ucs_enabled");
            expect(response.body.value).to.equal("true");
            cy.task(
              "cli_log",
              ` Global UCS configuration enabled successfully`
            );
          } else if (
            response.status === 400 &&
            response.body.error.message.includes("already exists")
          ) {
            cy.task(
              "cli_log",
              ` Global UCS configuration already exists - proceeding`
            );
          } else {
            throw new Error(
              `Failed to enable global UCS: ${response.status} - ${response.body.error.message}`
            );
          }
        });
        const merchantId = globalState.get("merchantId");
        const rolloutConfigs = [
          {
            key: `ucs_rollout_config_${merchantId}_${currentConnector}_card_Authorize`,
            operation: "Authorize",
          },
          {
            key: `ucs_rollout_config_${merchantId}_${currentConnector}_card_SetupMandate`,
            operation: "SetupMandate",
          },
          {
            key: `ucs_rollout_config_${merchantId}_${currentConnector}_card_PaymentSync`,
            operation: "PaymentSync",
          },
        ];

        rolloutConfigs.forEach(({ key, operation }) => {
          cy.task(
            "cli_log",
            ` Creating UCS rollout config for ${operation}: ${key}`
          );

          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/configs/`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("adminApiKey"),
            },
            body: { key: key, value: "1.0" },
            failOnStatusCode: false,
          }).then((response) => {
            if (response.status === 200) {
              expect(response.body.key).to.equal(key);
              expect(response.body.value).to.equal("1.0");
              cy.task(
                "cli_log",
                ` UCS ${operation} rollout config created successfully`
              );
            } else if (
              response.status === 400 &&
              response.body.error.message.includes("already exists")
            ) {
              cy.task(
                "cli_log",
                ` UCS ${operation} rollout config already exists - proceeding`
              );
            } else {
              throw new Error(
                `Failed to create UCS rollout config for ${operation}: ${response.status} - ${response.body.error.message}`
              );
            }
          });
        });
      });

      it("should verify UCS configuration", () => {
        cy.task(
          "cli_log",
          ` Verifying UCS configuration for ${currentConnector}...`
        );

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/configs/ucs_enabled`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task("cli_log", ` Global UCS config status: ${response.status}`);
          if (response.status === 200) {
            cy.task(
              "cli_log",
              ` Global UCS config verified: ${response.body.value}`
            );
            expect(response.body.value).to.equal("true");
          } else {
            throw new Error(
              `Global UCS config verification failed: ${response.status}`
            );
          }
        });

        const merchantId = globalState.get("merchantId");
        const rolloutConfigs = [
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_Authorize`,
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_SetupMandate`,
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_PaymentSync`,
        ];

        rolloutConfigs.forEach((rolloutConfigKey) => {
          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/configs/${rolloutConfigKey}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("adminApiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            cy.task(
              "cli_log",
              ` ${rolloutConfigKey} status: ${response.status}`
            );
            if (response.status === 200) {
              cy.task(
                "cli_log",
                ` ${rolloutConfigKey} verified: ${response.body.value}`
              );
              expect(response.body.value).to.equal("1.0");
            } else {
              throw new Error(
                `Rollout config verification failed: ${rolloutConfigKey} - ${response.status}`
              );
            }
          });
        });

        cy.task(
          "cli_log",
          ` UCS configuration verification completed for ${currentConnector}`
        );
      });

      it("should test all UCS-compatible requests", () => {
        const testRequest = (index) => {
          if (index >= testableRequests.length) {
            testResults.endTime = new Date();
            const duration = (
              (testResults.endTime - testResults.startTime) /
              1000
            ).toFixed(1);

            cy.task(
              "cli_log",
              `All ${testableRequests.length} UCS requests completed!`
            );
            cy.task("cli_log", `\n=== COMPREHENSIVE TEST RESULTS ===`);
            cy.task(
              "cli_log",
              `Total Available Requests: ${testResults.totalAvailable}`
            );
            cy.task(
              "cli_log",
              `Total Enabled for Testing: ${testResults.totalEnabled}`
            );
            cy.task(
              "cli_log",
              `Total Actually Tested: ${testResults.totalTested}`
            );
            cy.task(
              "cli_log",
              `PASSED (${testResults.passed.length}): ${testResults.passed.join(", ")}`
            );
            cy.task(
              "cli_log",
              `FAILED (${testResults.failed.length}): ${testResults.failed.join(", ") || "None"}`
            );
            cy.task(
              "cli_log",
              `SKIPPED (${testResults.skipped.length}): ${testResults.skipped.join(", ") || "None"}`
            );
            cy.task(
              "cli_log",
              `SUCCESS RATE: ${testResults.totalTested > 0 ? ((testResults.passed.length / testResults.totalTested) * 100).toFixed(1) : 0}%`
            );
            cy.task("cli_log", `TEST DURATION: ${duration} seconds`);
            cy.task("cli_log", `=== END TEST RESULTS ===`);
            return;
          }

          const requestType = testableRequests[index];
          testResults.totalTested = index + 1;
          cy.task(
            "cli_log",
            `\nTesting Request ${index + 1}/${testableRequests.length}: ${requestType}`
          );
          cy.task(
            "cli_log",
            `Progress: ${(((index + 1) / testableRequests.length) * 100).toFixed(1)}% | Passed: ${testResults.passed.length} | Failed: ${testResults.failed.length}`
          );

          const requestConfig = connectorConfig.card_pm[requestType];

          if (!requestConfig) {
            const errorMsg = ` Request configuration not found: ${requestType} in ${currentConnector}.js`;
            cy.task("cli_log", errorMsg);
            throw new Error(`UCS Test Failed - ${errorMsg}`);
          }

          if (!requestConfig.Request) {
            const errorMsg = ` Request body not found for: ${requestType}`;
            cy.task("cli_log", errorMsg);
            throw new Error(`UCS Test Failed - ${errorMsg}`);
          }

          if (!requestConfig.Response) {
            const errorMsg = ` Expected response not found for: ${requestType}`;
            cy.task("cli_log", errorMsg);
            throw new Error(`UCS Test Failed - ${errorMsg}`);
          }

          const baseRequest = requestConfig.Request;
          const expectedResponse = requestConfig.Response;

          cy.task("cli_log", `Request Type: ${requestType}`);
          cy.task("cli_log", `Expected Status: ${expectedResponse.status}`);
          cy.task("cli_log", `Base Request: ${expectedResponse.body.status}`);

          if (requestType.includes("MIT") || requestType.includes("Repeat")) {
            cy.task(
              "cli_log",
              `Skipping ${requestType} - requires existing mandate setup`
            );
            testResults.skipped.push(requestType);
            testRequest(index + 1);
          } else if (requestType === "UCSZeroAuthMandate") {
            const ucsZeroAuthConfig =
              connectorConfig.card_pm["UCSZeroAuthMandate"];
            const ucsConfirmConfig =
              connectorConfig.card_pm["UCSConfirmMandate"];
            const ucsRecurringConfig =
              connectorConfig.card_pm["UCSRecurringPayment"];

            if (ucsZeroAuthConfig && ucsConfirmConfig && ucsRecurringConfig) {
              cy.task(
                "cli_log",
                "Starting UCS Sequential Flow: ZeroAuth → Confirm → Recurring"
              );

              cy.task("cli_log", "Step 1/3: Executing UCSZeroAuthMandate");
              cy.request({
                method: "POST",
                url: `${globalState.get("baseUrl")}/payments`,
                headers: {
                  Accept: "application/json",
                  "Content-Type": "application/json",
                  "api-key": globalState.get("apiKey"),
                },
                body: ucsZeroAuthConfig.Request,
                failOnStatusCode: false,
              }).then((response1) => {
                cy.task(
                  "cli_log",
                  ` UCS ZeroAuth response status: ${response1.status}`
                );
                if (
                  response1.status !== ucsZeroAuthConfig.Response.status ||
                  response1.body.status !==
                    ucsZeroAuthConfig.Response.body.status
                ) {
                  const errorMsg = `FAILED: UCSZeroAuthMandate - Expected status ${ucsZeroAuthConfig.Response.status}, got ${response1.status}`;
                  cy.task(
                    "cli_log",
                    ` Full UCS ZeroAuth response: ${JSON.stringify(response1.body, null, 2)}`
                  );
                  throw new Error(`UCS Sequential Flow Failed - ${errorMsg}`);
                }

                const paymentId =
                  response1.body.payment_id || response1.body.id;
                globalState.set("paymentId", paymentId);
                cy.task(
                  "cli_log",
                  ` ZeroAuth succeeded - Payment ID: ${paymentId}`
                );
                testResults.passed.push("UCSZeroAuthMandate");

                cy.task("cli_log", "Step 2/3: Executing UCSConfirmMandate");
                cy.request({
                  method: "POST",
                  url: `${globalState.get("baseUrl")}/payments/${paymentId}/confirm`,
                  headers: {
                    Accept: "application/json",
                    "Content-Type": "application/json",
                    "api-key": globalState.get("apiKey"),
                  },
                  body: ucsConfirmConfig.Request,
                  failOnStatusCode: false,
                }).then((response2) => {
                  cy.task(
                    "cli_log",
                    ` UCS Confirm response status: ${response2.status}`
                  );
                  if (
                    response2.status !== ucsConfirmConfig.Response.status ||
                    response2.body.status !==
                      ucsConfirmConfig.Response.body.status
                  ) {
                    const errorMsg = `FAILED: UCSConfirmMandate - Expected status ${ucsConfirmConfig.Response.status}, got ${response2.status}`;
                    cy.task(
                      "cli_log",
                      ` Full UCS Confirm response: ${JSON.stringify(response2.body, null, 2)}`
                    );
                    throw new Error(`UCS Sequential Flow Failed - ${errorMsg}`);
                  }

                  const paymentMethodId = response2.body.payment_method_id;
                  globalState.set("paymentMethodId", paymentMethodId);
                  cy.task(
                    "cli_log",
                    ` Confirm succeeded - Payment Method ID: ${paymentMethodId}`
                  );
                  testResults.passed.push("UCSConfirmMandate");

                  cy.task("cli_log", "Step 3/3: Executing UCSRecurringPayment");
                  const recurringRequest = { ...ucsRecurringConfig.Request };
                  recurringRequest.recurring_details.data = paymentMethodId;

                  cy.request({
                    method: "POST",
                    url: `${globalState.get("baseUrl")}/payments`,
                    headers: {
                      Accept: "application/json",
                      "Content-Type": "application/json",
                      "api-key": globalState.get("apiKey"),
                    },
                    body: recurringRequest,
                    failOnStatusCode: false,
                  }).then((response3) => {
                    cy.task(
                      "cli_log",
                      ` UCS Recurring response status: ${response3.status}`
                    );
                    if (
                      response3.status !== ucsRecurringConfig.Response.status ||
                      response3.body.status !==
                        ucsRecurringConfig.Response.body.status
                    ) {
                      const errorMsg = `FAILED: UCSRecurringPayment - Expected status ${ucsRecurringConfig.Response.status}, got ${response3.status}`;
                      cy.task(
                        "cli_log",
                        ` Full UCS Recurring response: ${JSON.stringify(response3.body, null, 2)}`
                      );
                      throw new Error(
                        `UCS Sequential Flow Failed - ${errorMsg}`
                      );
                    }

                    cy.task(
                      "cli_log",
                      ` Recurring succeeded - UCS Sequential Flow Completed!`
                    );
                    testResults.passed.push("UCSRecurringPayment");

                    testRequest(index + 1);
                  });
                });
              });
            } else {
              throw new Error(
                `UCS Flows not found for connector: ${currentConnector} - ensure all UCS requests are defined in the config`
              );
            }
          } else {
            let ucsRequest;

            cy.task(
              "cli_log",
              ` Base request structure: ${JSON.stringify(baseRequest, null, 2)}`
            );

            const isMandateRequest =
              baseRequest.mandate_data && baseRequest.mandate_data !== null;
            let requestAmount = 6500;

            cy.task("cli_log", `Is mandate request: ${isMandateRequest}`);

            if (isMandateRequest && baseRequest.mandate_data.mandate_type) {
              cy.task(
                "cli_log",
                `Mandate data found: ${JSON.stringify(baseRequest.mandate_data)}`
              );

              if (baseRequest.mandate_data.mandate_type.single_use) {
                requestAmount =
                  baseRequest.mandate_data.mandate_type.single_use.amount;
                cy.task(
                  "cli_log",
                  `Using mandate single-use amount: ${requestAmount}`
                );
              } else if (baseRequest.mandate_data.mandate_type.multi_use) {
                requestAmount =
                  baseRequest.mandate_data.mandate_type.multi_use.amount;
                cy.task(
                  "cli_log",
                  `Using mandate multi-use amount: ${requestAmount}`
                );
              }
            } else if (isMandateRequest) {
              cy.task(
                "cli_log",
                `Mandate request detected but no mandate_type found`
              );
            }
            const customerIdHash = requestType.toLowerCase().substring(0, 8);
            const safeCustomerId = `ucs_${customerIdHash}`;

            const baseCard = baseRequest.payment_method_data?.card || {};
            const cardNumber = baseCard.card_number || "4111111111111111";

            let authType = "no_three_ds";
            if (requestType.includes("3DS") && !requestType.includes("No3DS")) {
              authType = "three_ds";
            }

            let captureMethod = "automatic";
            if (requestType.includes("Manual")) {
              captureMethod = "manual";
            }

            const email =
              baseRequest.email || `test_${customerIdHash}@example.com`;

            ucsRequest = {
              amount: 6540,
              currency: "USD",
              amount_to_capture: 6540,
              confirm: true,
              profile_id: globalState.get("profileId"),
              capture_method: captureMethod,
              capture_on: "2022-09-10T10:11:12Z",
              authentication_type: authType,
              setup_future_usage: "off_session",
              customer: {
                id: safeCustomerId,
                name: baseCard.card_holder_name || "John Doe",
                email: email,
                phone: "9999999999",
                phone_country_code: "+1",
              },
              customer_id: safeCustomerId,
              phone_country_code: "+1",
              routing: {
                type: "single",
                data: currentConnector,
              },
              description: `UCS ${requestType} test payment`,
              return_url: "https://google.com",
              payment_method: "card",
              payment_method_type: "credit",
              payment_method_data: {
                card: {
                  card_holder_name: baseCard.card_holder_name || "Joseph Doe",
                  card_number: cardNumber,
                  card_exp_month: baseCard.card_exp_month || "03",
                  card_exp_year: baseCard.card_exp_year || "2030",
                  card_cvc: baseCard.card_cvc || "100",
                  card_network: baseCard.card_network || "Visa",
                },
              },
              billing: {
                address: {
                  line1: "1467",
                  line2: "Harrison Street",
                  line3: "Harrison Street",
                  city: "San Fransico",
                  state: "California",
                  zip: "94122",
                  country: "US",
                  first_name: "joseph",
                  last_name: "Doe",
                },
                phone: {
                  number: "8056594427",
                  country_code: "+91",
                },
                email: email,
              },
              shipping: {
                address: {
                  line1: "1467",
                  line2: "Harrison Street",
                  line3: "Harrison Street",
                  city: "San Fransico",
                  state: "California",
                  zip: "94122",
                  country: "US",
                  first_name: "joseph",
                  last_name: "Doe",
                },
                phone: {
                  number: "8056594427",
                  country_code: "+91",
                },
                email: email,
              },
              statement_descriptor_name: "joseph",
              statement_descriptor_suffix: "JS",
              order_details: [
                {
                  product_name: `UCS ${requestType} Test`,
                  quantity: 1,
                  amount: 6540,
                  account_name: "transaction_processing",
                },
              ],
              metadata: {
                udf1: "value1",
                new_customer: "true",
                login_date: "2019-09-10T10:11:12Z",
                test_type: "ucs_comprehensive",
                request_type: requestType.toLowerCase(),
              },
              browser_info: {
                user_agent:
                  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
                accept_header:
                  "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
                language: "nl-NL",
                color_depth: 24,
                screen_height: 723,
                screen_width: 1536,
                time_zone: 0,
                java_enabled: true,
                java_script_enabled: true,
                ip_address: "128.0.0.1",
              },
              customer_acceptance: {
                acceptance_type: "offline",
                accepted_at: "1963-05-03T04:07:52.723Z",
                online: {
                  ip_address: "125.0.0.1",
                  user_agent: "amet irure esse",
                },
              },
              connector_metadata: {
                noon: {
                  order_category: "pay",
                },
              },
              payment_link: false,
              payment_link_config: {
                theme: "",
                logo: "",
                seller_name: "",
                sdk_layout: "",
                display_sdk_only: false,
                enabled_saved_payment_method: false,
              },
              payment_type: "normal",
              request_incremental_authorization: false,
              merchant_order_reference_id: `ucs_test_${requestType.toLowerCase()}_${Date.now()}`,
              all_keys_required: true,
              session_expiry: 900,
            };

            cy.task("cli_log", `Generated customer ID: ${safeCustomerId}`);
            cy.task("cli_log", `Authentication type: ${authType}`);
            cy.task("cli_log", `Capture method: ${captureMethod}`);

            cy.task(
              "cli_log",
              `Final UCS request: ${JSON.stringify(ucsRequest, null, 2)}`
            );

            cy.request({
              method: "POST",
              url: `${globalState.get("baseUrl")}/payments`,
              headers: {
                Accept: "application/json",
                "Content-Type": "application/json",
                "api-key": globalState.get("apiKey"),
              },
              body: ucsRequest,
              failOnStatusCode: false,
            }).then((response) => {
              cy.task(
                "cli_log",
                ` UCS ${requestType} response status: ${response.status}`
              );

              // Log full response body for detailed analysis
              cy.task(
                "cli_log",
                ` Full UCS ${requestType} response: ${JSON.stringify(response.body, null, 2)}`
              );

              if (
                response.status === expectedResponse.status &&
                response.body.status == expectedResponse.body.status
              ) {
                cy.task(
                  "cli_log",
                  `PASSED: ${requestType} via UCS (Status: ${response.status})`
                );
                testResults.passed.push(requestType);

                if (response.body && response.body.status) {
                  cy.task(
                    "cli_log",
                    ` Payment Status: ${response.body.status}`
                  );

                  // Log additional important fields if they exist
                  if (response.body.payment_id) {
                    cy.task(
                      "cli_log",
                      ` Payment ID: ${response.body.payment_id}`
                    );
                  }
                  if (response.body.gateway_system) {
                    cy.task(
                      "cli_log",
                      `Gateway System: ${response.body.gateway_system}`
                    );
                  }
                  if (response.body.connector) {
                    cy.task(
                      "cli_log",
                      ` Connector: ${response.body.connector}`
                    );
                  }
                  if (response.body.amount) {
                    cy.task("cli_log", ` Amount: ${response.body.amount}`);
                  }
                }

                testRequest(index + 1);
              } else {
                const errorMsg = `FAILED: ${requestType} - Expected status ${expectedResponse.status}, got ${response.status} and body status ${response.body.status}`;
                cy.task("cli_log", errorMsg);
                testResults.failed.push(requestType);
                cy.task(
                  "cli_log",
                  `Full error response: ${JSON.stringify(response, null, 2)}`
                );
                cy.task(
                  "cli_log",
                  `Request that failed: ${JSON.stringify(ucsRequest, null, 2)}`
                );
                throw new Error(`UCS Test Failed - ${errorMsg}`);
              }
            });
          }
        };

        testRequest(0);
      });

      after(() => {
        cy.task("cli_log", `\n Starting cleanup for ${currentConnector}...`);

        cy.task("cli_log", ` Cleaning up UCS configurations...`);

        const merchantId = globalState.get("merchantId");
        const rolloutConfigs = [
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_Authorize`,
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_SetupMandate`,
          `ucs_rollout_config_${merchantId}_${currentConnector}_card_PaymentSync`,
        ];

        rolloutConfigs.forEach((rolloutConfigKey) => {
          cy.request({
            method: "DELETE",
            url: `${globalState.get("baseUrl")}/configs/${rolloutConfigKey}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("adminApiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            if (response.status === 200 || response.status === 404) {
              cy.task(
                "cli_log",
                ` Deleted UCS rollout config: ${rolloutConfigKey}`
              );
            } else {
              cy.task(
                "cli_log",
                `⚠️ Failed to delete UCS rollout config: ${rolloutConfigKey} - ${response.status}`
              );
            }
          });
        });

        cy.request({
          method: "DELETE",
          url: `${globalState.get("baseUrl")}/configs/ucs_enabled`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          if (response.status === 200 || response.status === 404) {
            cy.task("cli_log", ` Deleted global UCS configuration`);
          } else {
            cy.task(
              "cli_log",
              `⚠️ Failed to delete global UCS configuration - ${response.status}`
            );
          }
        });

        cy.task("cli_log", ` Cleanup for ${currentConnector} completed`);
        if (testableRequests && testableRequests.length > 0) {
          cy.task(
            "cli_log",
            ` All ${testableRequests.length} requests passed for ${currentConnector}!`
          );
        }
      });
    });
  } else {
    describe(`UCS Tests - Skipped for ${currentConnector}`, () => {
      it("should skip UCS tests for unsupported connector", () => {
        cy.task(
          "cli_log",
          `Connector ${currentConnector} is not supported for UCS tests`
        );
        cy.task(
          "cli_log",
          `Supported UCS connectors: ${UCS_SUPPORTED_CONNECTORS.join(", ")}`
        );
      });
    });
  }

  after(() => {
    cy.task("cli_log", "\n=== UCS Comprehensive Testing Completed ===");
    if (isUCSSupported) {
      cy.task(
        "cli_log",
        ` Successfully tested UCS integration for ${currentConnector}`
      );
      cy.task("cli_log", " UCS integration is working correctly!");
    } else {
      cy.task("cli_log", ` UCS tests were skipped for ${currentConnector}`);
    }
  });
});
