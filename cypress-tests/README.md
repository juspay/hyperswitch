# Hyperswitch Cypress Testing Framework

## Overview

This is a comprehensive testing framework built with [Cypress](https://cypress.io) to automate testing for [Hyperswitch](https://github.com/juspay/hyperswitch/). The framework supports API testing with features like multiple credential management, configuration management, global state handling, and extensive utility functions. The framework provides extensive support for API testing with advanced features including:

- Multiple credential management
- Dynamic configuration management
- Global state handling
- Extensive utility functions
- Parallel test execution
- Connector-specific implementations

## Table of Contents

- [Overview](#overview)
- [Table of Contents](#table-of-contents)
- [Quick Start](#quick-start)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Running Tests](#running-tests)
    - [Development Mode (Interactive)](#development-mode-interactive)
    - [CI Mode (Headless)](#ci-mode-headless)
    - [Execute tests against multiple connectors or in parallel](#execute-tests-against-multiple-connectors-or-in-parallel)
- [Test reports](#test-reports)
- [Folder structure](#folder-structure)
- [Adding tests](#adding-tests)
  - [Addition of test for a new connector](#addition-of-test-for-a-new-connector)
  - [Developing Core Features or adding new tests](#developing-core-features-or-adding-new-tests)
    - [1. Create or update test file](#1-create-or-update-test-file)
    - [2. Add New Commands](#2-add-new-commands)
    - [Managing global state](#managing-global-state)
- [Debugging](#debugging)
  - [1. Interactive Mode](#1-interactive-mode)
  - [2. Logging](#2-logging)
  - [3. Screenshots](#3-screenshots)
  - [4. State Debugging](#4-state-debugging)
  - [5. Hooks](#5-hooks)
  - [6. Tasks](#6-tasks)
- [Linting](#linting)
- [Best Practices](#best-practices)
- [Additional Resources](#additional-resources)
- [Contributing](#contributing)
- [Example creds.json](#example-credsjson)

## Quick Start

For experienced users who want to get started quickly:

```bash
git clone https://github.com/juspay/hyperswitch.git
cd cypress-tests
npm ci
CYPRESS_CONNECTOR="connector_id" npm run cypress:ci
```

## Getting Started

## Prerequisites

- Node.js (18.x or above)
- npm or yarn
- [Hyperswitch development environment](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md)

> [!NOTE]
> To learn about the hardware requirements and software dependencies for running Cypress, refer to the [official documentation](https://docs.cypress.io/app/get-started/install-cypress).

## Installation

1. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hyperswitch.git
   cd cypress-tests
   ```

2. Install Cypress and its dependencies to `cypress-tests` directory by running the following command:

   ```shell
   npm ci
   ```

   Once installed, verify the installation by running:

   ```shell
    npx cypress --version
   ```

   To learn about the supported commands, execute:

   ```shell
   npm run
   ```

3. Set up the cards database:

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

> [!TIP]
> It is recommended to install [direnv](https://github.com/direnv/direnv) and use a `.envrc` file to store these environment variables with `cypress-tests` directory. This will make it easier to manage environment variables while working with Cypress tests.

> [!NOTE]
> To learn about how `creds` file should be structured, refer to the [example.creds.json](#example-credsjson) section below.

## Running Tests

Execution of Cypress tests can be done in two modes: Development mode (Interactive) and CI mode (Headless). The tests can be executed against a single connector or multiple connectors in parallel. Time taken to execute the tests will vary based on the number of connectors and the number of tests. For a single connector, the tests will take approximately 07-12 minutes to execute.

### Development Mode (Interactive)

```bash
npm run cypress
```

### CI Mode (Headless)

```bash
# All tests
npm run cypress:ci

# Specific test suites
npm run cypress:payments            # Payment tests
npm run cypress:payment-method-list # Payment method list tests
npm run cypress:payouts             # Payout tests
npm run cypress:routing             # Routing tests
```

### Execute tests against multiple connectors or in parallel

1. Set additional environment variables:

   ```shell
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

## Test reports

The test reports are generated in the `cypress/reports` directory. The reports are generated in the `mochawesome` format and can be viewed in the browser.
These reports does include:

- screenshots of the failed tests
- HTML and JSON reports

## Folder structure

The folder structure of this directory is as follows:

```txt
.
├── .prettierrc                       # prettier configs
├── README.md                         # this file
├── cypress
│   ├── e2e
│   │   ├── <Service>Test             # Directory for test scenarios related to connectors.
│   │   │   ├── 00000-test_<0>.cy.js
│   │   │   ├── ...
│   │   │   └── 0000n-test_<n>.cy.js
│   │   └── <Service>Utils            # Directory for utility functions related to connectors.
│   │        ├── connector_<1>.js
│   │        ├── ...
│   │        └── connector_<n>.js
│   ├── fixtures                      # Directory for storing test data API request.
│   │   ├── fixture_<1>.json
│   │   ├── ...
│   │   └── fixture_<n>.json
│   ├── support                       # Directory for Cypress support files.
│   │   ├── commands.js               # File containing custom Cypress commands and utilities.
│   │   ├── e2e.js
│   │   └── redirectionHandler.js
│   └── utils
│       ├── RequestBodyUtils.js
│       ├── State.js
│       └── featureFlags.js
├── cypress.config.js                 # Cypress configuration file.
├── eslint.config.js                  # linter configuration file.
└── package.json                      # Node.js package file.
```

## Adding tests

### Addition of test for a new connector

1. Include the connector details in the `creds.json` file

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

3. In `Utils.js`, import the new connector details

4. If the connector has a specific redirection requirement, add relevant redirection logic in `support/redirectionHandler.js`

### Developing Core Features or adding new tests

#### 1. Create or update test file

To add a new test, create a new test file in the `e2e` directory under respective `service`. The test file should follow the naming convention `000<number>-<Service>Test.cy.js` and should contain the test cases related to the service.

```javascript
// cypress/e2e/<Service>Test/NewFeature.cy.js
import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";

describe("New Feature", () => {
  let globalState;

  before(() => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("tests new functionality", () => {
    // Test implementation
  });
});
```

### 2. Add New Commands

```javascript
// cypress/support/commands.js
Cypress.Commands.add("newCommand", (params, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  const apiKey = globalState.get("apiKey");
  const url = `${baseUrl}/endpoint`;

  cy.request({
    method: "POST",
    url: url,
    headers: {
      "api-key": apiKey,
    },
    body: params,
  }).then((response) => {
    // Assertions
  });
});
```

### Managing global state

The global state is used to share data between tests. The global state is stored in the `State` class and is accessible across all tests. Can only be accessed in the `before` and `after` hooks.

## Debugging

### 1. Interactive Mode

- Use `npm run cypress` for real-time test execution
- View request/response details in Cypress UI
- Use DevTools for deeper debugging

### 2. Logging

```javascript
cy.task("cli_log", "Debug message");
cy.log("Test state:", globalState.data);
```

### 3. Screenshots

- Automatically captured on test failure
- Custom screenshot capture:

```javascript
cy.screenshot("debug-state");
```

### 4. State Debugging

- Add state logging in hooks:

```javascript
beforeEach(() => {
  cy.log("Current state:", JSON.stringify(globalState.data));
});
```

### 5. Hooks

- If the `globalState` object does not contain latest data, it must be due to the hooks not being executed in the correct order
- Add `cy.log(globalState)` to the test case to verify the data in the `globalState` object

> [!NOTE]
> Refer to the Cypress's official documentation for more information on hooks and their execution order [here](https://docs.cypress.io/app/core-concepts/writing-and-organizing-tests#Hooks).

### 6. Tasks

- Use `cy.task` to interact with the Node.js environment
- Task can only be used in `support` files and `spec` files. Using them in files outside these directories will result in unexpected behavior or errors like abrupt termination of the test suite

````javascript

## Linting

To run the formatting and lint checks, execute the following command:

```shell
# Format the code
npm run format

# Check the formatting
npm run format:check

# Lint the code. This wont fix the logic issues, unused imports or variables
npm run lint -- --fix
````

## Best Practices

1. Use the global state for sharing data between tests
2. Implement proper error handling
3. Use appropriate wait strategies
4. Maintain test independence
5. Follow the existing folder structure
6. Document connector-specific behaviors
7. Use descriptive test and variable names
8. Use custom commands for repetitive tasks
9. Use `cy.log` for debugging and do not use `console.log`

## Additional Resources

- [Cypress Documentation](https://docs.cypress.io/)
- [API Testing Best Practices](https://docs.cypress.io/guides/end-to-end-testing/api-testing)
- [Hyperswitch API Documentation](https://hyperswitch.io/docs)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests following the guidelines
4. Submit a pull request

## Example creds.json

```json
{
  // Connector with single credential support and metadata support
  "adyen": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "api_key",
      "key1": "key1",
      "api_secret": "api_secret"
    },
    "metadata": {
      "key": "value"
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
  // Connector with multiple credential support
  "cybersource": {
    "connector_1": {
      "connector_account_details": {
        "auth_type": "SignatureKey",
        "api_key": "api_key",
        "key1": "key1",
        "api_secret": "api_secret"
      }
    },
    "connector_2": {
      "connector_account_details": {
        "auth_type": "SignatureKey",
        "api_key": "api_key",
        "key1": "key1",
        "api_secret": "api_secret"
      }
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
