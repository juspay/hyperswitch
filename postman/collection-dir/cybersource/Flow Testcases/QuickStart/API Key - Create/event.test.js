// Validate status 2xx
pm.test("[POST]::/api_keys/:merchant_id - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/api_keys/:merchant_id - Content-Type is application/json",
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

// pm.collectionVariables - Set api_key_id as variable for jsonData.key_id
if (jsonData?.key_id) {
  pm.collectionVariables.set("api_key_id", jsonData.key_id);
  console.log(
    "- use {{api_key_id}} as collection variable for value",
    jsonData.key_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{api_key_id}}, as jsonData.key_id is undefined.",
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
