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

// Response body should have value "Succeeded" for "status"
pm.test(
"[POST]::/payments/:id - Content check if value for 'status' matches 'requires_payment_method'",
    function () {
        pm.expect(jsonData.status).to.eql("requires_payment_method");
    },
);

pm.test(
  "[POST]::/payments - Content check if no 'connector' is affected to the payment intent",
  function () {
    pm.expect(jsonData.connector).to.be.null;
  },
);

// Response body should not have "connector_transaction_id"
pm.test(
  "[POST]::/payments - Content check if 'connector_transaction_id' is null",
  function () {
    pm.expect(jsonData.connector_transaction_id).to.be.null;
  },
);
