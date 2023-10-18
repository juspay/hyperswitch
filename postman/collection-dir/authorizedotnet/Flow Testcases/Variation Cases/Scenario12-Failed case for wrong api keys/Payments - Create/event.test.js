// Validate status 2xx
pm.test("[GET]::/payments/:id - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payments/:id - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[GET]::/payments/:id - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set payment_id as variable for jsonData.payment_id
if (jsonData?.payment_id) {
  pm.collectionVariables.set("payment_id", jsonData.payment_id);
  console.log(
    "- use {{payment_id}} as collection variable for value",
    jsonData.payment_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{payment_id}}, as jsonData.payment_id is undefined.",
  );
}

// pm.collectionVariables - Set mandate_id as variable for jsonData.mandate_id
if (jsonData?.mandate_id) {
  pm.collectionVariables.set("mandate_id", jsonData.mandate_id);
  console.log(
    "- use {{mandate_id}} as collection variable for value",
    jsonData.mandate_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{mandate_id}}, as jsonData.mandate_id is undefined.",
  );
}

// pm.collectionVariables - Set client_secret as variable for jsonData.client_secret
if (jsonData?.client_secret) {
  pm.collectionVariables.set("client_secret", jsonData.client_secret);
  console.log(
    "- use {{client_secret}} as collection variable for value",
    jsonData.client_secret,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{client_secret}}, as jsonData.client_secret is undefined.",
  );
}


// Response body should have value "processing" or "failed" for "error_code"
if (jsonData?.error_code) {
  pm.test(
    "[POST]::/payments - Content check if value for 'error_code' matches 'E00007' or '11' ",
    function () {
      pm.expect(jsonData.error_code).to.be.oneOf(["E00007", "11"]);
    },
  );
}

// Response body should have value "processing" or "failed" for "status"
if (jsonData?.error_message) {
  pm.test(
    "[POST]::/payments - Content check if value for 'error_message' matches 'processing' or 'This transaction has been declined.' ",
    function () {
      pm.expect(jsonData.error_message).to.be.oneOf(["A duplicate transaction has been submitted.", "User authentication failed due to invalid authentication values."]);
    },
  );
}

// Response body should have value "processing" or "failed" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'processing' or 'failed' ",
    function () {
      pm.expect(jsonData.status).to.be.oneOf(["failed"]);
    },
  );
}
