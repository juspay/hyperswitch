# Cypress Tests

## Overview
This Tool is a solution designed to automate testing for the [Hyperswitch](https://github.com/juspay/hyperswitch/) using Cypress, an open-source tool capable of conducting API call tests and UI tests. This README provides guidance on installing Cypress and its dependencies and outlines how to utilize the HS Test Automation Tool effectively.

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
3. Clone the repository and switch to the project directory:

   ```shell
   git clone https://github.com/juspay/hs-test-automation
   cd hs-test-automation
   ```
4. Set the cypress env
   ```shell
   export CYPRESS_CONNECTOR="connector_id"
   export CYPRESS_BASEURL="base_url"
   export DEBUG=cypress:cli
   export CYPRESS_ADMINAPIKEY="admin_api_key"
   ```

5. Run Cypress test cases
    To run the tests in a browser in interactive mode run the following command
    ```
    npm run cypress 
    ```
    To run the tests in headless mode run the following command
    ```
    npm run cypress:ci
    ```

## Additional Resources
For more information on using Cypress and writing effective tests, refer to the official Cypress documentation: [Cypress Documentation](https://docs.cypress.io/)
