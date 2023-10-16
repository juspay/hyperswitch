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

// Response body should have value "succeeded" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]:://payments/:id/capture - Content check if value for 'status' matches 'succeeded'",
    function () {
      pm.expect(jsonData.status).to.eql("succeeded");
    },
  );
}

// Validate the connector
pm.test("[POST]::/payments - connector", function () {
  pm.expect(jsonData.connector).to.eql("checkout");
});

// Response body should have value "6540" for "amount"
if (jsonData?.amount) {
  pm.test(
    "[post]:://payments/:id/capture - Content check if value for 'amount' matches '6540'",
    function () {
      pm.expect(jsonData.amount).to.eql(6540);
    },
  );
}

// Response body should have value "6000" for "amount_received"
if (jsonData?.amount_received) {
  pm.test(
    "[POST]::/payments:id/capture - Content check if value for 'amount_received' matches '6000'",
    function () {
      pm.expect(jsonData.amount_received).to.eql(6000);
    },
  );
}

// Response body should have value "6540" for "amount_capturable"
if (jsonData?.amount_capturable) {
  pm.test(
    "[post]:://payments/:id/capture - Content check if value for 'amount_capturable' matches 'amount - 540'",
    function () {
      pm.expect(jsonData.amount_capturable).to.eql(6540);
    },
  );
}
