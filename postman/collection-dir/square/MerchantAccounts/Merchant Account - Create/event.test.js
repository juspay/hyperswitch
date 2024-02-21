// Validate status 2xx
pm.test("[POST]::/accounts - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/accounts - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) { }

// pm.collectionVariables - Set merchant_id as variable for jsonData.merchant_id
if (jsonData?.merchant_id) {
  pm.collectionVariables.set("merchant_id", jsonData.merchant_id);
  console.log(
    "- use {{merchant_id}} as collection variable for value",
    jsonData.merchant_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{merchant_id}}, as jsonData.merchant_id is undefined.",
  );
}

// pm.collectionVariables - Set api_key as variable for jsonData.api_key
if (jsonData?.api_key) {
  pm.collectionVariables.set("api_key", jsonData.api_key);
  console.log(
    "- use {{api_key}} as collection variable for value",
    jsonData.api_key,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{api_key}}, as jsonData.api_key is undefined.",
  );
}

// pm.collectionVariables - Set publishable_key as variable for jsonData.publishable_key
if (jsonData?.publishable_key) {
  pm.collectionVariables.set("publishable_key", jsonData.publishable_key);
  console.log(
    "- use {{publishable_key}} as collection variable for value",
    jsonData.publishable_key,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{publishable_key}}, as jsonData.publishable_key is undefined.",
  );
}

// pm.collectionVariables - Set merchant_id as variable for jsonData.merchant_id
if (jsonData?.merchant_id) {
  pm.collectionVariables.set("organization_id", jsonData.organization_id);
  console.log(
    "- use {{organization_id}} as collection variable for value",
    jsonData.organization_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{organization_id}}, as jsonData.organization_id is undefined.",
  );
}

// Response body should have "mandate_id"
pm.test(
  "[POST]::/accounts - Organization id is generated",
  function () {
    pm.expect(typeof jsonData.organization_id !== "undefined").to.be.true;
  },
);
