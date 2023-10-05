// Validate status 2xx
pm.test(
  "[POST]::/account/:account_id/connectors - Status code is 2xx",
  function () {
    pm.response.to.be.success;
  },
);

// Validate if response header has matching content-type
pm.test(
  "[POST]::/account/:account_id/connectors - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set merchant_connector_id as variable for jsonData.merchant_connector_id
if (jsonData?.merchant_connector_id) {
  pm.collectionVariables.set(
    "merchant_connector_id",
    jsonData.merchant_connector_id,
  );
  console.log(
    "- use {{merchant_connector_id}} as collection variable for value",
    jsonData.merchant_connector_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{merchant_connector_id}}, as jsonData.merchant_connector_id is undefined.",
  );
}

// Validate the auth_type and its credentials
pm.test(
  "[POST]::/account/:account_id/connectors - Content check if auth_type is BodyKey",
  function () {
    pm.expect(typeof jsonData.connector_account_details.auth_type !== "BodyKey")
      .to.be.true;
  },
);

pm.test(
  "[POST]::/account/:account_id/connectors - Content check if 'api_key' exists",
  function () {
    pm.expect(typeof jsonData.connector_account_details.api_key !== "undefined")
      .to.be.true;
  },
);

pm.test(
  "[POST]::/account/:account_id/connectors - Content check if 'key1' exists",
  function () {
    pm.expect(typeof jsonData.connector_account_details.key1 !== "undefined").to
      .be.true;
  },
);
