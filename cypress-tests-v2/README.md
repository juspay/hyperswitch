# Cypress Tests

## Overview

This tool is a comprehensive testing solution designed to automate testing for [Hyperswitch](https://github.com/juspay/hyperswitch/) using Cypress, an open-source testing framework capable of conducting both API and UI tests. This README provides detailed guidance on installing Cypress, configuring the environment, and writing effective tests.

## Table of Contents

- [Installation](#installation)
- [Environment Configuration](#environment-configuration)
- [Running Tests](#running-tests)
- [Folder Structure](#folder-structure)
- [Writing Tests](#writing-tests)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)
- [Additional Resources](#additional-resources)

## Installation

### Prerequisites

Before installing Cypress, ensure you have the following prerequisites installed:

- **Node.js** (18.x and above) - [Download here](https://nodejs.org/)
- **npm** (Node Package Manager) - Usually comes with Node.js
- **Git** - For cloning the repository

### Quick Start

To set up and run test cases on your local machine, follow these steps:

1. **Clone the repository and navigate to the project directory:**

   ```shell
   git clone https://github.com/juspay/hyperswitch
   cd hyperswitch/cypress-tests-v2
   ```

2. **Install Cypress and dependencies:**

   ```shell
   npm install
   ```

3. **Set up environment variables** (see [Environment Configuration](#environment-configuration) below)

4. **Run tests** (see [Running Tests](#running-tests) below)

## Environment Configuration

### Required Environment Variables

Before running tests, you must set the following environment variables:

```shell
export CYPRESS_CONNECTOR="stripe"                                    # Connector ID to test
export CYPRESS_BASEURL="http://localhost:8080"                      # Hyperswitch server base URL
export CYPRESS_ADMINAPIKEY="your_admin_api_key_here"               # Admin API key
export CYPRESS_CONNECTOR_AUTH_FILE_PATH="/path/to/creds.json"      # Path to connector credentials
export DEBUG=cypress:cli                                           # Enable Cypress CLI debugging
```

### Setting Up Credentials File

Create a `creds.json` file with your connector credentials. See the [Example creds.json](#example-credsjson) section for the required format.

**Example:**
```json
{
  "stripe": {
    "auth_type": "HeaderKey",
    "api_key": "sk_test_your_stripe_key_here"
  }
}
```

### Alternative: Using cypress.env.json

You can also set environment variables using a `cypress.env.json` file in the project root:

```json
{
  "CONNECTOR": "stripe",
  "BASEURL": "http://localhost:8080",
  "ADMINAPIKEY": "your_admin_api_key_here",
  "CONNECTOR_AUTH_FILE_PATH": "/path/to/creds.json"
}
```

## Running Tests

### Interactive Mode (Recommended for Development)

Run tests with the Cypress Test Runner GUI:

```shell
npm run cypress
```

This opens the Cypress Test Runner where you can:
- Select and run individual test files
- See tests execute in real-time
- Debug test failures
- View network requests and responses

### Headless Mode (CI/CD)

Run all tests in headless mode (no GUI):

```shell
npm run cypress:ci
```

### Specific Test Suites

Run specific categories of tests:

```shell
# Payment-related tests
npm run cypress:payments

# Payout-related tests  
npm run cypress:payouts

# Routing-related tests
npm run cypress:routing
```

### Running Individual Test Files

To run a specific test file:

```shell
npx cypress run --spec "cypress/e2e/ConnectorTest/YourTestFile.cy.js"
```

### Running Tests with Specific Browser

```shell
npx cypress run --browser chrome
npx cypress run --browser firefox
npx cypress run --browser edge
```

> [!NOTE]
> To learn about how the credentials file should be structured, refer to the [Example creds.json](#example-credsjson) section below.

## Folder Structure

The folder structure of this directory is organized as follows:

```text
cypress-tests-v2/                                       # Root directory for Cypress tests
├── .gitignore                                          # Git ignore rules
├── .prettierrc.json                                    # Code formatting configuration
├── cypress.config.js                                   # Cypress configuration file
├── package.json                                        # Node.js dependencies and scripts
├── package-lock.json                                   # Locked dependency versions
├── README.md                                           # This documentation file
└── cypress/                                            # Cypress test files and configuration
    ├── e2e/                                            # End-to-end test directory
    │   ├── ConnectorTest/                              # Connector-specific test scenarios
    │   │   ├── PaymentTest/                            # Payment flow tests
    │   │   ├── PayoutTest/                             # Payout flow tests
    │   │   ├── RoutingTest/                            # Routing logic tests
    │   │   └── *.cy.js                                 # Individual test files
    │   └── ConnectorUtils/                             # Utility functions for connectors
    │       ├── Stripe.js                               # Stripe connector utilities
    │       ├── PayPal.js                               # PayPal connector utilities
    │       └── utils.js                                # Common utility functions
    ├── fixtures/                                       # Test data and API request templates
    │   ├── payment-request.json                        # Sample payment requests
    │   └── connector-configs.json                      # Connector configuration data
    ├── support/                                        # Cypress support files
    │   ├── commands.js                                 # Custom Cypress commands
    │   └── e2e.js                                      # Global test configuration
    └── utils/                                          # General utility functions
        └── helpers.js                                  # Helper functions for tests
```

## Writing Tests

### Adding Connectors

To add a new payment connector for testing:

#### 1. Add Connector Credentials

Include the connector details in your `creds.json` file:

```json
{
  "new_connector": {
    "auth_type": "HeaderKey",           // or "BodyKey", "SignatureKey"
    "api_key": "your_api_key",
    "key1": "additional_key_if_needed", // Optional
    "api_secret": "your_secret"         // For SignatureKey auth_type
  }
}
```

#### 2. Create Connector Utility File

Create a new file in `cypress/e2e/ConnectorUtils/` named after your connector (e.g., `NewConnector.js`):

```javascript
// cypress/e2e/ConnectorUtils/NewConnector.js

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2025",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
};

export default {
  connectorDetails,
  successfulNo3DSCardDetails,
};
```

#### 3. Import in Utils

Add your connector to `cypress/e2e/ConnectorUtils/utils.js`:

```javascript
import NewConnector from "./NewConnector.js";

// Add to the getConnectorDetails function
export const getConnectorDetails = (connectorId) => {
  switch (connectorId) {
    case "new_connector":
      return NewConnector.connectorDetails;
    // ... other cases
  }
};
```

### Adding Custom Commands

Add helper functions in `cypress/support/commands.js`:

```javascript
Cypress.Commands.add("createPaymentIntent", (globalState, data) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: data,
  }).then((response) => {
    expect(response.status).to.eq(200);
    globalState.set("paymentIntentId", response.body.payment_id);
    return cy.wrap(response.body);
  });
});
```

### Adding Test Scenarios

Create new test files in `cypress/e2e/ConnectorTest/`:

```javascript
// cypress/e2e/ConnectorTest/NewConnectorPayment.cy.js

import { connectorDetails } from "../ConnectorUtils/utils.js";

describe("New Connector Payment Tests", () => {
  let globalState;

  beforeEach(() => {
    globalState = new Map();
    globalState.set("connectorId", Cypress.env("CONNECTOR"));
    globalState.set("baseUrl", Cypress.env("BASEURL"));
    globalState.set("apiKey", Cypress.env("ADMINAPIKEY"));
  });

  it("should create payment intent successfully", () => {
    const data = connectorDetails.card_pm.PaymentIntent.Request;
    
    cy.createPaymentIntent(globalState, data).then((response) => {
      expect(response.status).to.eq("requires_payment_method");
      expect(response.payment_id).to.exist;
    });
  });

  it("should complete payment successfully", () => {
    // Test implementation here
  });
});
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Environment Variables Not Set

**Problem:** Tests fail with "undefined" errors for environment variables.

**Solution:** 
- Verify all required environment variables are set
- Check that variable names match exactly (case-sensitive)
- Use `echo $CYPRESS_BASEURL` to verify variables are set

#### 2. Credentials File Not Found

**Problem:** Error "ENOENT: no such file or directory" when loading creds.json.

**Solution:**
- Verify the file path in `CYPRESS_CONNECTOR_AUTH_FILE_PATH`
- Use absolute path instead of relative path
- Check file permissions

#### 3. Connection Refused Errors

**Problem:** Tests fail with "ECONNREFUSED" errors.

**Solution:**
- Ensure Hyperswitch server is running on the specified base URL
- Check if the port number is correct
- Verify network connectivity

#### 4. API Key Authentication Failures

**Problem:** Tests fail with 401 Unauthorized errors.

**Solution:**
- Verify the admin API key is correct and active
- Check that the API key has sufficient permissions
- Ensure the API key format matches expectations

#### 5. Cypress Binary Issues

**Problem:** "Cypress binary not found" or corruption errors.

**Solution:**
```shell
# Clear Cypress cache and reinstall
npx cypress cache clear
npm uninstall cypress
npm install cypress
```

#### 6. Browser Launch Issues

**Problem:** Cypress cannot launch the browser.

**Solution:**
- Update your browser to the latest version
- Try a different browser: `npx cypress run --browser firefox`
- Check for conflicting browser extensions

### Debug Mode

Enable detailed logging:

```shell
export DEBUG=cypress:*
npm run cypress
```

### Getting Help

If you encounter issues:

1. Check the [Cypress documentation](https://docs.cypress.io/)
2. Look for similar issues in the [Hyperswitch GitHub issues](https://github.com/juspay/hyperswitch/issues)
3. Join the [Hyperswitch Slack community](https://join.slack.com/t/hyperswitch-io/shared_invite/zt-2awm23agh-p_G5xNpziv6yAiedTkkqLg)

## Best Practices

### Test Organization

- **Use descriptive test names** that explain what is being tested
- **Group related tests** using `describe` blocks
- **Use `beforeEach`** for common setup code
- **Clean up after tests** to avoid side effects

### Writing Maintainable Tests

```javascript
// ✅ Good: Descriptive and focused
describe("Payment Intent Creation", () => {
  it("should create payment intent with valid card details", () => {
    // Test implementation
  });
});

// ❌ Bad: Vague and unclear
describe("Test", () => {
  it("works", () => {
    // Test implementation
  });
});
```

### Data Management

- **Use fixtures** for complex test data
- **Store reusable data** in connector utility files
- **Use environment variables** for configuration that changes between environments

### Assertions

```javascript
// ✅ Good: Specific assertions
expect(response.status).to.eq(200);
expect(response.body.payment_id).to.exist;
expect(response.body.status).to.eq("requires_payment_method");

// ❌ Bad: Generic assertions
expect(response).to.exist;
```

### Error Handling

```javascript
// Handle API errors gracefully
cy.request({
  method: "POST",
  url: "/payments",
  body: paymentData,
  failOnStatusCode: false, // Don't fail immediately on non-2xx status
}).then((response) => {
  if (response.status === 400) {
    // Handle expected error case
    expect(response.body.error).to.exist;
  } else {
    // Handle success case
    expect(response.status).to.eq(200);
  }
});
```

## Additional Resources

- [Cypress Documentation](https://docs.cypress.io/) - Official Cypress docs
- [Cypress Best Practices](https://docs.cypress.io/guides/references/best-practices) - Official best practices guide
- [Hyperswitch API Documentation](https://docs.hyperswitch.io/) - API reference
- [Hyperswitch GitHub Repository](https://github.com/juspay/hyperswitch) - Source code and issues

## Example creds.json

Below is a comprehensive example of the credentials file structure for different connector types:

```json
{
  "adyen": {
    "auth_type": "SignatureKey",
    "api_key": "your_adyen_api_key",
    "key1": "your_adyen_merchant_account",
    "api_secret": "your_adyen_api_secret"
  },
  "bankofamerica": {
    "auth_type": "SignatureKey",
    "api_key": "your_boa_api_key",
    "key1": "your_boa_key1",
    "api_secret": "your_boa_api_secret"
  },
  "bluesnap": {
    "auth_type": "BodyKey",
    "api_key": "your_bluesnap_api_key",
    "key1": "your_bluesnap_key1"
  },
  "cybersource": {
    "auth_type": "SignatureKey",
    "api_key": "your_cybersource_api_key",
    "key1": "your_cybersource_key1",
    "api_secret": "your_cybersource_api_secret"
  },
  "nmi": {
    "auth_type": "BodyKey",
    "api_key": "your_nmi_api_key",
    "key1": "your_nmi_key1"
  },
  "paypal": {
    "auth_type": "BodyKey",
    "api_key": "your_paypal_client_id",
    "key1": "your_paypal_client_secret"
  },
  "stripe": {
    "auth_type": "HeaderKey",
    "api_key": "sk_test_your_stripe_secret_key"
  },
  "trustpay": {
    "auth_type": "SignatureKey",
    "api_key": "your_trustpay_api_key",
    "key1": "your_trustpay_key1",
    "api_secret": "your_trustpay_api_secret"
  }
}
```

### Auth Type Explanations

- **HeaderKey**: API key is sent in the request header
- **BodyKey**: API key is sent in the request body
- **SignatureKey**: Requires API key, additional key, and secret for signature generation

---

**Contributing:** If you find issues with this documentation or have suggestions for improvement, please feel free to open an issue or submit a pull request to the [Hyperswitch repository](https://github.com/juspay/hyperswitch).
