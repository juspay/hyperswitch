# Cypress Tests

## Overview

This Tool is a solution designed to automate testing for the [Hyperswitch](https://github.com/juspay/hyperswitch/) using Cypress, an open-source tool capable of conducting API call tests and UI tests. This README provides guidance on installing Cypress and its dependencies.

## Installation

### Prerequisites

Before installing Cypress, ensure you have the following prerequisites installed:

- npm (Node Package Manager)
- Node.js (18.x and above)

### Run Test Cases on your local

To run test cases, follow these steps:

1. Install Cypress

   ```shell
   npm install cypress --save-dev
   ```

2. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hyperswitch
   cd cypress-tests
   ```

3. Set environment variables for cypress

   ```shell
   export CYPRESS_CONNECTOR="connector_id"
   export CYPRESS_BASEURL="base_url"
   export DEBUG=cypress:cli
   export CYPRESS_ADMINAPIKEY="admin_api_key"
   ```

4. Run Cypress test cases

   To execute a connector create test, ensure the connector details are included in the `creds.json` file. Then, integrate the path in the `command.js` as follows:

   ```javascript
   import ConnectorAuthDetails from "../../creds.json";
   ```

   To run the tests in a browser in interactive mode run the following command

   ```shell
   npm run cypress
   ```

   To run the tests in headless mode run the following command

   ```shell
   npm run cypress:ci
   ```

## Folder Structure

The folder structure of this directory is as follows:

```text
.                                                        # The root directory for the Cypress tests.
├── .gitignore
├── cypress                                              # Contains Cypress-related files and folders.
│   ├── e2e                                              # End-to-end test directory.
│   │   ├── ConnectorTest                                # Directory for test scenarios related to connectors.
│   │   │   ├── your_testcase1_files_here.cy.js
│   │   │   ├── your_testcase2_files_here.cy.js
│   │   │   └── ...
│   │   └── ConnectorUtils                               # Directory for utility functions related to connectors.
│   │       ├── connector_detail_files_here.js
│   │       └── utils.js
│   ├── fixtures                                         # Directory for storing test data API request.
│   │   └── your_fixture_files_here.json
│   ├── support                                          # Directory for Cypress support files.
│   │   ├── commands.js                                  # File containing custom Cypress commands and utilities.
│   │   └── e2e.js
│   └── utils
│       └── utility_files_go_here.js
├── cypress.config.js                                    # Cypress configuration file.
├── cypress.env.json                                     # File is used to store environment-specific configuration values,such as base URLs, which can be accessed within your Cypress tests.
├── package.json                                         # Node.js package file.
├── readme.md                                            # This file
└── yarn.lock
```

## Writing Tests

### Adding Connectors

To add a new connector for testing with Hyperswitch, follow these steps:

1.Include the connector details in the `creds.json` file:

example:

```json
{
  "stripe": {
    "auth_type": "HeaderKey",
    "api_key": "SK_134"
  }
}
```

2.Add the new connector details to the ConnectorUtils folder (including CardNo and connector-specific information).

Refer to Stripe.js file for guidance:

```javascript
/cypress-tests/cypress/e2e/ConnectorUtils/Stripe.js
```

Similarly, create a new file named newconnectorname.js and include all the relevant information for that connector.

3.In util.js, import the new connector details.

### Adding Functions

Similarly, add any helper functions or utilities in the `command.js` in support folder and import them into your tests as needed.

Example: Adding List Mandate function to support `ListMandate` scenario

```javascript
Cypress.Commands.add("listMandateCallTest", (globalState) => {
  const customerId = globalState.get("customerId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customerId}/mandates`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    const xRequestId = response.headers["x-request-id"];
    if (xRequestId) {
      cy.task("cli_log", "x-request-id ->> " + xRequestId);
    } else {
      cy.task(
        "cli_log",
        "x-request-id is not available in the response headers"
      );
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    let i = 0;
    for (i in response.body) {
      if (response.body[i].mandate_id === globalState.get("mandateId")) {
        expect(response.body[i].status).to.equal("active");
      }
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

You can create similar scenarios by calling other functions defined in `command.js`. These functions interact with utility files like `connector.js` and include necessary assertions to support various connector scenarios.

## Additional Resources

For more information on using Cypress and writing effective tests, refer to the official Cypress documentation: [Cypress Documentation](https://docs.cypress.io/)
