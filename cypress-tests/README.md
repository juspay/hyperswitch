# Cypress Tests

## Overview

This Tool is a solution designed to automate testing for the [Hyperswitch](https://github.com/juspay/hyperswitch/) using Cypress, an open-source tool capable of conducting API call tests and UI tests. This README provides guidance on installing Cypress and its dependencies.

## Installation

### Prerequisites

Before installing Cypress, ensure that `Node` and `npm` is installed on your machine. To check if it is installed, run the following command:

```shell
node -v
npm -v
```

If not, download and install `Node` from the official [Node.js website](https://nodejs.org/en/download/package-manager/current). This will also install `npm`.

### Run Test Cases on your local

To run test cases, follow these steps:

1. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hyperswitch
   cd cypress-tests
   ```

2. Install Cypress and its dependencies to `cypress-tests` directory by running the following command:

   ```shell
   npm ci
   ```

3. Insert data to `cards_info` table in `hyperswitch_db`

   ```shell
   psql --host=localhost --port=5432 --username=db_user --dbname=hyperswitch_db --command "\copy cards_info FROM '.github/data/cards_info.csv' DELIMITER ',' CSV HEADER;"
   ```

4. Set environment variables for cypress

   ```shell
   export CYPRESS_CONNECTOR="connector_id"
   export CYPRESS_BASEURL="base_url"
   export DEBUG=cypress:cli
   export CYPRESS_ADMINAPIKEY="admin_api_key"
   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="path/to/creds.json"
   ```

5. Run Cypress test cases

   To run the tests in interactive mode run the following command

   ```shell
   npm run cypress
   ```

   To run all the tests in headless mode run the following command

   ```shell
   npm run cypress:ci
   ```

   To run payment tests in headless mode run the following command

   ```shell
   npm run cypress:payments
   ```

   To run payout tests in headless mode run the following command

   ```shell
   npm run cypress:payouts
   ```

   To run routing tests in headless mode run the following command

   ```shell
   npm run cypress:routing
   ```

In order to run cypress tests against multiple connectors at a time or in parallel:

1. Set up `.env` file that exports necessary info:

   ```env
   export DEBUG=cypress:cli

   export CYPRESS_ADMINAPIKEY='admin_api_key'
   export CYPRESS_BASEURL='base_url'
   export CYPRESS_CONNECTOR_AUTH_FILE_PATH="path/to/creds.json"

   export PAYMENTS_CONNECTORS="payment_connector_1 payment_connector_2 payment_connector_3 payment_connector_4"
   export PAYOUTS_CONNECTORS="payout_connector_1 payout_connector_2 payout_connector_3"
   export PAYMENT_METHOD_LIST=""
   export ROUTING=""
   ```

2. In terminal, execute:

   ```shell
   source .env
   scripts/execute_cypress.sh
   ```

   Optionally, `--parallel <jobs (integer)>` can be passed to run cypress tests in parallel. By default, when `parallel` command is passed, it will be run in batches of `5`.

> [!NOTE]
> To learn about how creds file should be structured, refer to the [example.creds.json](#example-credsjson) section below.

## Folder Structure

The folder structure of this directory is as follows:

```text
.                                                        # The root directory for the Cypress tests.
├── .gitignore
├── cypress                                              # Contains Cypress-related files and folders.
│   ├── e2e                                              # End-to-end test directory.
│   │   ├── ConnectorTest                                # Directory for test scenarios related to connectors.
│   │   │   ├── your_testcase1_files_here.cy.js
│   │   │   ├── your_testcase2_files_here.cy.js
│   │   │   └── ...
│   │   └── ConnectorUtils                               # Directory for utility functions related to connectors.
│   │       ├── connector_detail_files_here.js
│   │       └── utils.js
│   ├── fixtures                                         # Directory for storing test data API request.
│   │   └── your_fixture_files_here.json
│   ├── support                                          # Directory for Cypress support files.
│   │   ├── commands.js                                  # File containing custom Cypress commands and utilities.
│   │   └── e2e.js
│   └── utils
│       └── utility_files_go_here.js
├── cypress.config.js                                    # Cypress configuration file.
├── cypress.env.json                                     # File is used to store environment-specific configuration values,such as base URLs, which can be accessed within your Cypress tests.
├── package.json                                         # Node.js package file.
├── readme.md                                            # This file
└── yarn.lock
```

## Writing Tests

### Adding Connectors

To add a new connector for testing with Hyperswitch, follow these steps:

1. Include the connector details in the `creds.json` file:

   example:

   ```json
   {
     "stripe": {
       "connector_account_details": {
         "auth_type": "HeaderKey",
         "api_key": "SK_134"
       }
     }
   }
   ```

2. Add the new connector details to the ConnectorUtils folder (including CardNo and connector-specific information).

   Refer to Stripe.js file for guidance:

   ```javascript
   /cypress-tests/cypress/e2e/ConnectorUtils/Stripe.js
   ```

   **File Naming:** Create a new file named <connector_name>.js for your specific connector.

   **Include Relevant Information:** Populate the file with all the necessary details specific to that connector.

   **Handling Unsupported Features:**

   - If a connector does not support a specific payment method or feature:
   - You can omit the relevant configurations in the <connector_name>.js file.
   - The handling of unsupported features will be managed by the commons.js file, which will throw an unsupported or not implemented error as appropriate.

3. In `Utils.js`, import the new connector details.

### Adding Functions

Similarly, add any helper functions or utilities in the `commands.js` in support folder and import them into your tests as needed.

Example: Adding List Mandate function to support `ListMandate` scenario

```javascript
Cypress.Commands.add("listMandateCallTest", (globalState) => {
  // declare all the variables and constants
  const customerId = globalState.get("customerId");
  // construct the URL for the API call
  const url: `${globalState.get("baseUrl")}/customers/${customerId}/mandates`
  const api_key = globalState.get("apiKey");

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
    },
    // set failOnStatusCode to false to prevent Cypress from failing the test
    failOnStatusCode: false,
  }).then((response) => {
    // mandatorliy log the `x-request-id` to the console
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");

    if (response.status === 200) {
      // do the necessary validations like below
      for (const key in response.body) {
        expect(response.body[key]).to.have.property("mandate_id");
        expect(response.body[key]).to.have.property("status");
      }
    } else {
      // handle the error response
      expect(response.status).to.equal(400);
    }
  });
});
```

### Adding Scenarios

To add new test scenarios:

1. Navigate to the ConnectorTest directory.
2. Create a new test file or modify existing ones to add your scenarios.
3. Write your test scenarios using Cypress commands.

For example, to add a scenario for listing mandates in the `Mandateflows`:

```javascript
// cypress/ConnectorTest/CreateSingleuseMandate.js
describe("Payment Scenarios", () => {
  it("should complete a successful payment", () => {
    // Your test logic here
  });
});
```

In this scenario, you can call functions defined in `command.js`. For instance, to test the `listMandateCallTest` function:

```javascript
describe("Payment Scenarios", () => {
  it("list-mandate-call-test", () => {
    cy.listMandateCallTest(globalState);
  });
});
```

You can create similar scenarios by calling other functions defined in `commands.js`. These functions interact with utility files like `<connector_name>.js` and include necessary assertions to support various connector scenarios.

### Debugging

It is recommended to run `npm run cypress` while developing new test cases to debug and verify as it opens the Cypress UI allowing the developer to run individual tests. This also opens up the possibility to to view the test execution in real-time and debug any issues that may arise by viewing the request and response payloads directly.

If, for any reason, the `globalState` object does not contain latest data, it must be due to the hooks not being executed in the correct order. In such cases, it is recommended to add `cy.log(globalState)` to the test case to verify the data in the `globalState` object.
Please refer to the Cypress's official documentation for more information on hooks and their execution order [here](https://docs.cypress.io/app/core-concepts/writing-and-organizing-tests#Hooks).

## Additional Resources

For more information on using Cypress and writing effective tests, refer to the official Cypress documentation: [Cypress Documentation](https://docs.cypress.io/)

## Example creds.json

```json
{
  "adyen": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "api_key",
      "key1": "key1",
      "api_secret": "api_secret"
    }
  },
  "bankofamerica": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "api_key",
      "key1": "key1",
      "api_secret": "api_secret"
    }
  },
  "bluesnap": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "api_key",
      "key1": "key1"
    }
  },
  "cybersource": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "api_key",
      "key1": "key1",
      "api_secret": "api_secret"
    }
  },
  "nmi": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "api_key",
      "key1": "key1"
    }
  },
  "paypal": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "api_key",
      "key1": "key1"
    }
  },
  "stripe": {
    "connector_account_details": {
      "auth_type": "HeaderKey",
      "api_key": "api_key"
    }
  },
  "trustpay": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "api_key",
      "key1": "key1",
      "api_secret": "api_secret"
    }
  }
}
```
