// Validate status 2xx or 4xx
pm.test("[POST]::/payments/:id/capture - Status code is 2xx", function () {
  pm.expect(pm.response.code).to.be.oneOf([200, 400]);
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/cancel - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/cancel - Response has JSON Body", function () {
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

// Response body should have value "processing" or "failed" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'processing' or 'failed' ",
    function () {
      pm.expect(jsonData.status).to.be.oneOf(["processing", "cancelled"]);
    },
  );
}

// Response body should have an error message as we try to capture a "processing" payment
if (jsonData?.error) {
  pm.test(
    "[POST]::/payments - Content check if error type is 'invalid_request'",
    function () {
      pm.expect(jsonData.error.message).to.be.oneOf([
        "You cannot cancelled this payment because it has status processing",
        "You cannot cancelled this payment because it has status failed",
      ]);
    },
  );
}
