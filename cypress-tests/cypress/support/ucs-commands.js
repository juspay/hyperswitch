// UCS-specific Cypress commands to reduce code duplication

/**
 * Sets up all UCS configurations for a given connector
 * @param {Object} globalState - Global state object
 * @param {string} connector - Connector name (e.g., 'authorizedotnet')
 */
Cypress.Commands.add("setupUCSConfigs", (globalState, connector) => {
  cy.task("cli_log", "🔧 Enabling global UCS configuration...");

  // Enable global UCS configuration with proper error handling
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
      cy.task("cli_log", "✅ Global UCS configuration enabled successfully");
    } else if (
      response.status === 400 &&
      response.body.error.message.includes("already exists")
    ) {
      cy.task(
        "cli_log",
        "✅ Global UCS configuration already exists - proceeding"
      );
    } else {
      throw new Error(
        `Failed to enable global UCS: ${response.status} - ${response.body.error.message}`
      );
    }
  });

  // Create rollout configurations
  const merchantId = globalState.get("merchantId");
  const rolloutConfigs = [
    {
      key: `ucs_rollout_config_${merchantId}_${connector}_card_Authorize`,
      operation: "Authorize",
    },
    {
      key: `ucs_rollout_config_${merchantId}_${connector}_card_SetupMandate`,
      operation: "SetupMandate",
    },
    {
      key: `ucs_rollout_config_${merchantId}_${connector}_card_PaymentSync`,
      operation: "PaymentSync",
    },
  ];

  rolloutConfigs.forEach(({ key, operation }) => {
    cy.task(
      "cli_log",
      `📝 Creating UCS rollout config for ${operation}: ${key}`
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
          `✅ UCS ${operation} rollout config created successfully`
        );
      } else if (
        response.status === 400 &&
        response.body.error.message.includes("already exists")
      ) {
        cy.task(
          "cli_log",
          `✅ UCS ${operation} rollout config already exists - proceeding`
        );
      } else {
        throw new Error(
          `Failed to create UCS rollout config for ${operation}: ${response.status} - ${response.body.error.message}`
        );
      }
    });
  });
});

/**
 * Verifies all UCS configurations are properly set
 * @param {Object} globalState - Global state object
 * @param {string} connector - Connector name
 */
Cypress.Commands.add("verifyUCSConfigs", (globalState, connector) => {
  cy.task("cli_log", `🔍 Verifying UCS configuration for ${connector}...`);

  // Verify global UCS config
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/configs/ucs_enabled`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    cy.task("cli_log", `📊 Global UCS config status: ${response.status}`);
    if (response.status === 200) {
      cy.task(
        "cli_log",
        `✅ Global UCS config verified: ${response.body.value}`
      );
      expect(response.body.value).to.equal("true");
    } else {
      throw new Error(
        `Global UCS config verification failed: ${response.status}`
      );
    }
  });

  // Verify rollout configs
  const merchantId = globalState.get("merchantId");
  const rolloutConfigs = [
    `ucs_rollout_config_${merchantId}_${connector}_card_Authorize`,
    `ucs_rollout_config_${merchantId}_${connector}_card_SetupMandate`,
    `ucs_rollout_config_${merchantId}_${connector}_card_PaymentSync`,
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
      cy.task("cli_log", `📊 ${rolloutConfigKey} status: ${response.status}`);
      if (response.status === 200) {
        cy.task(
          "cli_log",
          `✅ ${rolloutConfigKey} verified: ${response.body.value}`
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
    `🎉 UCS configuration verification completed for ${connector}`
  );
});

/**
 * Cleans up all UCS configurations
 * @param {Object} globalState - Global state object
 * @param {string} connector - Connector name
 */
Cypress.Commands.add("cleanupUCSConfigs", (globalState, connector) => {
  cy.task("cli_log", `🧹 Starting cleanup for ${connector}...`);
  cy.task("cli_log", "🗑️ Cleaning up UCS configurations...");

  const merchantId = globalState.get("merchantId");
  const rolloutConfigs = [
    `ucs_rollout_config_${merchantId}_${connector}_card_Authorize`,
    `ucs_rollout_config_${merchantId}_${connector}_card_SetupMandate`,
    `ucs_rollout_config_${merchantId}_${connector}_card_PaymentSync`,
  ];

  // Delete rollout configs
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
          `🗑️ Deleted UCS rollout config: ${rolloutConfigKey}`
        );
      } else {
        cy.task(
          "cli_log",
          `⚠️ Failed to delete UCS rollout config: ${rolloutConfigKey} - ${response.status}`
        );
      }
    });
  });

  // Delete global UCS config
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
      cy.task("cli_log", "🗑️ Deleted global UCS configuration");
    } else {
      cy.task(
        "cli_log",
        `⚠️ Failed to delete global UCS configuration - ${response.status}`
      );
    }
  });

  cy.task("cli_log", `✨ Cleanup for ${connector} completed`);
});

/**
 * Creates a UCS payment request with standard structure
 * @param {string} requestType - Type of UCS request
 * @param {string} connector - Connector name
 * @param {Object} globalState - Global state object
 * @param {Object} baseRequest - Base request from connector config
 */
Cypress.Commands.add(
  "createUCSPayment",
  (requestType, connector, globalState, baseRequest) => {
    cy.task(
      "cli_log",
      `📋 Base request structure: ${JSON.stringify(baseRequest, null, 2)}`
    );

    // Check for mandate request logic from original
    const isMandateRequest =
      baseRequest.mandate_data && baseRequest.mandate_data !== null;
    let requestAmount = 6500;

    cy.task("cli_log", `📊 Is mandate request: ${isMandateRequest}`);

    if (isMandateRequest && baseRequest.mandate_data.mandate_type) {
      cy.task(
        "cli_log",
        `📝 Mandate data found: ${JSON.stringify(baseRequest.mandate_data)}`
      );

      if (baseRequest.mandate_data.mandate_type.single_use) {
        requestAmount = baseRequest.mandate_data.mandate_type.single_use.amount;
        cy.task(
          "cli_log",
          `💰 Using mandate single-use amount: ${requestAmount}`
        );
      } else if (baseRequest.mandate_data.mandate_type.multi_use) {
        requestAmount = baseRequest.mandate_data.mandate_type.multi_use.amount;
        cy.task(
          "cli_log",
          `💰 Using mandate multi-use amount: ${requestAmount}`
        );
      }
    } else if (isMandateRequest) {
      cy.task(
        "cli_log",
        `⚠️ Mandate request detected but no mandate_type found`
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

    const email = baseRequest.email || `test_${customerIdHash}@example.com`;

    const ucsPaymentBody = {
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
        data: connector,
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

    cy.task("cli_log", `🚀 Generated customer ID: ${safeCustomerId}`);
    cy.task("cli_log", `🔐 Authentication type: ${authType}`);
    cy.task("cli_log", `💰 Capture method: ${captureMethod}`);

    cy.task(
      "cli_log",
      `📄 Final UCS request: ${JSON.stringify(ucsPaymentBody, null, 2)}`
    );

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      body: ucsPaymentBody,
      failOnStatusCode: false,
    });
  }
);

/**
 * Validates UCS payment response
 * @param {Object} response - Cypress response object
 * @param {Object} expectedResponse - Expected response structure
 * @param {string} requestType - Type of UCS request
 */
Cypress.Commands.add(
  "validateUCSResponse",
  (response, expectedResponse, requestType) => {
    cy.task(
      "cli_log",
      `🔍 UCS ${requestType} response status: ${response.status}`
    );

    // Log full response body for detailed analysis (matching original)
    cy.task(
      "cli_log",
      `📄 Full UCS ${requestType} response: ${JSON.stringify(response.body, null, 2)}`
    );

    if (
      response.status === expectedResponse.status &&
      response.body.status == expectedResponse.body.status
    ) {
      cy.task(
        "cli_log",
        `✅ PASSED: ${requestType} via UCS (Status: ${response.status})`
      );

      if (response.body && response.body.status) {
        cy.task("cli_log", `📊 Payment Status: ${response.body.status}`);

        // Log additional important fields if they exist (matching original)
        if (response.body.payment_id) {
          cy.task("cli_log", `💳 Payment ID: ${response.body.payment_id}`);
        }
        if (response.body.gateway_system) {
          cy.task(
            "cli_log",
            `🌐 Gateway System: ${response.body.gateway_system}`
          );
        }
        if (response.body.connector) {
          cy.task("cli_log", `🔌 Connector: ${response.body.connector}`);
        }
        if (response.body.amount) {
          cy.task("cli_log", `💰 Amount: ${response.body.amount}`);
        }
      }

      cy.wrap({ success: true, requestType });
    } else {
      const errorMsg = `❌ FAILED: ${requestType} - Expected status ${expectedResponse.status}, got ${response.status} and body status ${response.body.status}`;
      cy.task("cli_log", errorMsg);
      cy.task(
        "cli_log",
        `📄 Full error response: ${JSON.stringify(response, null, 2)}`
      );

      cy.wrap({ success: false, requestType, error: errorMsg });
    }
  }
);

/**
 * Logs test progress and results
 * @param {number} current - Current test index
 * @param {number} total - Total number of tests
 * @param {Array} passed - Array of passed tests
 * @param {Array} failed - Array of failed tests
 */
Cypress.Commands.add("logTestProgress", (current, total, passed, failed) => {
  const progress = (((current + 1) / total) * 100).toFixed(1);
  cy.task(
    "cli_log",
    `📊 Progress: ${progress}% | Passed: ${passed.length} | Failed: ${failed.length}`
  );
});

/**
 * Logs comprehensive test results
 * @param {Object} testResults - Test results object
 */
Cypress.Commands.add("logTestResults", (testResults) => {
  const duration = (
    (testResults.endTime - testResults.startTime) /
    1000
  ).toFixed(1);

  cy.task(
    "cli_log",
    `🏁 All ${testResults.totalTested} UCS requests completed!`
  );
  cy.task("cli_log", `\n=== 📊 COMPREHENSIVE TEST RESULTS ===`);
  cy.task(
    "cli_log",
    `📋 Total Available Requests: ${testResults.totalAvailable}`
  );
  cy.task(
    "cli_log",
    `✅ Total Enabled for Testing: ${testResults.totalEnabled}`
  );
  cy.task("cli_log", `🧪 Total Actually Tested: ${testResults.totalTested}`);
  cy.task(
    "cli_log",
    `🎉 PASSED (${testResults.passed.length}): ${testResults.passed.join(", ")}`
  );
  cy.task(
    "cli_log",
    `❌ FAILED (${testResults.failed.length}): ${testResults.failed.join(", ") || "None"}`
  );
  cy.task(
    "cli_log",
    `⏭️ SKIPPED (${testResults.skipped.length}): ${testResults.skipped.join(", ") || "None"}`
  );

  const successRate =
    testResults.totalTested > 0
      ? ((testResults.passed.length / testResults.totalTested) * 100).toFixed(1)
      : 0;
  cy.task("cli_log", `📈 SUCCESS RATE: ${successRate}%`);
  cy.task("cli_log", `⏱️ TEST DURATION: ${duration} seconds`);
  cy.task("cli_log", `=== 🏁 END TEST RESULTS ===`);
});

/**
 * Sets up complete UCS test environment (merchant + connector + configs)
 * @param {Object} fixtures - Test fixtures
 * @param {Object} globalState - Global state object
 * @param {string} connector - Connector name
 */
Cypress.Commands.add(
  "setupUCSEnvironment",
  (fixtures, globalState, connector) => {
    cy.task("cli_log", `🛠️ Setting up UCS environment for ${connector}...`);

    // Setup merchant account and connector using existing commands
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      fixtures.payment_methods_enabled,
      globalState
    );

    // Setup UCS configurations
    cy.setupUCSConfigs(globalState, connector);
    cy.verifyUCSConfigs(globalState, connector);

    cy.task("cli_log", `✅ UCS environment setup completed for ${connector}`);
  }
);

/**
 * Loads and validates UCS connector configuration
 * @param {string} connector - Connector name
 * @param {Array} ucsRequestNames - List of UCS request names to filter
 * @param {Function} getConnectorDetails - Connector details function
 * @returns {Object} Configuration object with testable requests
 */
Cypress.Commands.add(
  "loadUCSConfig",
  (connector, ucsRequestNames, getConnectorDetails) => {
    const config = getConnectorDetails(connector).card_pm;
    const connectorConfig = { card_pm: config };

    if (!connectorConfig?.card_pm) {
      throw new Error(
        `Failed to load configuration for connector: ${connector}`
      );
    }

    const allRequests = Object.keys(connectorConfig.card_pm);
    const testableRequests = allRequests.filter((requestType) =>
      ucsRequestNames.includes(requestType)
    );

    // Log coverage information
    cy.task("cli_log", `📦 Loading connector configuration for: ${connector}`);
    cy.task(
      "cli_log",
      `📊 Total requests available in ${connector}.js: ${allRequests.length}`
    );
    cy.task(
      "cli_log",
      `✅ UCS-compatible requests found: ${testableRequests.length}`
    );
    cy.task("cli_log", `📝 Testable requests: ${testableRequests.join(", ")}`);
    cy.task(
      "cli_log",
      `📈 Test Coverage: ${testableRequests.length}/${allRequests.length} (${((testableRequests.length / allRequests.length) * 100).toFixed(1)}%)`
    );

    if (testableRequests.length === 0) {
      throw new Error(
        `No UCS-compatible requests found for connector: ${connector}`
      );
    }

    // Store results in globalState or return them
    cy.wrap({
      connectorConfig,
      testableRequests,
      totalAvailable: allRequests.length,
      totalEnabled: testableRequests.length,
    });
  }
);

/**
 * Executes the UCS Sequential Flow: ZeroAuth → Confirm → Recurring
 * @param {Object} connectorConfig - Connector configuration object
 * @param {Object} testResults - Test results tracking object
 * @param {string} currentConnector - Current connector name
 * @param {Object} globalState - Global state object
 */
Cypress.Commands.add(
  "executeUCSSequentialFlow",
  (connectorConfig, testResults, currentConnector, globalState) => {
    const ucsZeroAuthConfig = connectorConfig.card_pm["UCSZeroAuthMandate"];
    const ucsConfirmConfig = connectorConfig.card_pm["UCSConfirmMandate"];
    const ucsRecurringConfig = connectorConfig.card_pm["UCSRecurringPayment"];

    if (!ucsZeroAuthConfig || !ucsConfirmConfig || !ucsRecurringConfig) {
      throw new Error(
        `❌ UCS Sequential Flow configs not found for connector: ${currentConnector}`
      );
    }

    cy.task(
      "cli_log",
      "🔄 Starting UCS Sequential Flow: ZeroAuth → Confirm → Recurring"
    );

    // Step 1: ZeroAuth
    cy.task("cli_log", "1️⃣ Step 1/3: Executing UCSZeroAuthMandate");
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
      cy.validateUCSResponse(
        response1,
        ucsZeroAuthConfig.Response,
        "UCSZeroAuthMandate"
      ).then((result1) => {
        if (!result1.success) {
          throw new Error(`UCS Sequential Flow Failed - ${result1.error}`);
        }

        if (testResults.passed) {
          testResults.passed.push("UCSZeroAuthMandate");
        }
        const paymentId = response1.body.payment_id || response1.body.id;
        globalState.set("paymentId", paymentId);

        // Step 2: Confirm
        cy.task("cli_log", "2️⃣ Step 2/3: Executing UCSConfirmMandate");
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
          cy.validateUCSResponse(
            response2,
            ucsConfirmConfig.Response,
            "UCSConfirmMandate"
          ).then((result2) => {
            if (!result2.success) {
              throw new Error(`UCS Sequential Flow Failed - ${result2.error}`);
            }

            if (testResults.passed) {
              testResults.passed.push("UCSConfirmMandate");
            }
            const paymentMethodId = response2.body.payment_method_id;
            globalState.set("paymentMethodId", paymentMethodId);

            // Step 3: Recurring
            cy.task("cli_log", "3️⃣ Step 3/3: Executing UCSRecurringPayment");
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
              cy.validateUCSResponse(
                response3,
                ucsRecurringConfig.Response,
                "UCSRecurringPayment"
              ).then((result3) => {
                if (!result3.success) {
                  throw new Error(
                    `UCS Sequential Flow Failed - ${result3.error}`
                  );
                }

                if (testResults.passed) {
                  testResults.passed.push("UCSRecurringPayment");
                }
                cy.task(
                  "cli_log",
                  "🎉 UCS Sequential Flow Completed Successfully!"
                );
              });
            });
          });
        });
      });
    });
  }
);
