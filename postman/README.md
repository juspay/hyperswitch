# Postman Collection

This directory contains the Postman collection for all Hyperswitch supported connectors and this documentation talks about how to build the collection.

## Development of collections

### Prerequisites

- [Postman](https://www.postman.com/downloads/)
- Newman CLI fork from npm: npm install -g 'https://github.com/knutties/newman.git#feature/newman-dir'

### Steps to build the collection

- `stripe.postman_collection.json` is the most up-to-date collection that consists of all the features that Hyperswitch supports
- It is recommended that you use the `stripe.postman_collection.json` as the base collection to build the collection for other connectors
- If you developed a new feature, make sure you add them to the `stripe.postman_collection.json` given that it is a core feature. If it is collection specific, add it to the respective collection

---

- The collection consists of many directories and each directory consists of a set of requests. Each directory is a feature and each request is a test case
- The directory name should be the name of the feature and the request name should be the name of the test case
- If the feature that you add is a flow test case, make sure you add the test case to the `Flow Testcases` directory. If you did a refactor that handles errors say, expiry date of a card, make sure you add the test case to the `Variation Cases` directory prefixed by `Scenario-<number>` where `<number>` is the number of the scenario
- If the feature that you add is a core feature, make sure you add the test case to the `Happy Cases` directory where only the happy cases are tested.

---

- Make sure that you update the `tests` section where the necessary `javascript` code has to written/updated to test the feature (assertion checks where you verify the results obtained with the expected outcome)
- If certain `tests` need to be run at the time of making a request, make sure you add them to the `Pre-request Script` section of the request

---

- After all the development is done, make sure you right click and run the collection in respective environments to make sure that the collection runs successfully
- Export the collection as `v2.1` and save it `postman/collection-json` directory
- Export the postman-collection to its directory structure by using the command `newman dir-export /path/to/collection.json` and move the folder to `postman/collection-dir` (for more info, refer to [Newman-Fork](https://github.com/juspay/hyperswitch/tree/main/crates/test_utils#newman))
- You can run the dir postman collection from newman using `rustman` by referring [here](https://github.com/juspay/hyperswitch/tree/main/crates/test_utils#running-tests)