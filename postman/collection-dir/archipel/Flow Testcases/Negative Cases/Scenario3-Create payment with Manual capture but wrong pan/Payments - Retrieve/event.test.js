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

// Response body should have value "failed" for "status"
pm.test(
"[POST]::/payments/:id - Content check if value for 'status' matches 'failed'",
    function () {
        pm.expect(jsonData.status).to.eql("failed");
    },
);

// Response body should have "connector_transaction_id"
pm.test(
  "[POST]::/payments - Content check if 'connector_transaction_id' exists",
  function () {
    pm.expect(typeof jsonData.connector_transaction_id !== "undefined").to.be
      .true;
  },
);

let payment_attempt = {}

pm.test(
  "[POST]::/payments - Payment has one 'Payment Attempt'",
  function () {
    pm.expect(jsonData.attempts.length).to.be.equal(1);
    payment_attempt = jsonData.attempts[0]
  },
);

pm.test(
  "[POST]::/payments - Payment Attempt has 'authorization_failed' status",
  function () {
    pm.expect(payment_attempt.status).to.be.equal("authorization_failed");
  },
);


pm.test(
  "[POST]::/payments - Payment Attempt has 'connector_transaction_id'",
  function () {
    pm.expect(payment_attempt.connector_transaction_id).to.be.equal(
        pm.collectionVariables.get("connector_transaction_id")
    );
  },
);

pm.test(
  "[POST]::/payments - Payment Attempt has archiepl response 'transactionId' in connector_metadata",
  function () {
    pm.expect(payment_attempt.connector_metadata.transactionId).to.be.not.null;
    pm.collectionVariables.set("archipel_transaction_uuid", payment_attempt.connector_metadata.transactionId)
  },
);

pm.test(
  "[POST]::/payments - Payment Attempt has no error",
  function () {
    pm.expect(payment_attempt.error_message).to.be.null;
  },
);
