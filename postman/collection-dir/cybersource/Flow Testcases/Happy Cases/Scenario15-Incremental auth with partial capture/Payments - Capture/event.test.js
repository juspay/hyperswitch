// Validate status 2xx
pm.test("[POST]::/payments/:id/capture - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/capture - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/capture - Response has JSON Body", function () {
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

// Response body should have value "partially_captured" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]:://payments/:id/capture - Content check if value for 'status' matches 'partially_captured'",
    function () {
      pm.expect(jsonData.status).to.eql("partially_captured");
    },
  );
}

// Response body should have value "1001" for "amount"
if (jsonData?.amount) {
  pm.test(
    "[post]:://payments/:id/capture - Content check if value for 'amount' matches '1001'",
    function () {
      pm.expect(jsonData.amount).to.eql(1001);
    },
  );
}

// Response body should have value "500" for "amount_received"
if (jsonData?.amount_received) {
  pm.test(
    "[POST]::/payments:id/capture - Content check if value for 'amount_received' matches '500'",
    function () {
      pm.expect(jsonData.amount_received).to.eql(500);
    },
  );
}
