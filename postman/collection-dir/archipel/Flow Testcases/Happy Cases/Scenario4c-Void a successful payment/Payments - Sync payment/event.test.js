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
    "[POST]::/payments/:id - Content check if value for 'status' matches 'succeeded'",
    function () {
        pm.expect(jsonData.status).to.eql("succeeded");
    },
);

pm.test(
  "[POST]::/payments/:id - Content check if value for 'amount' equal 500",
  function () {
    pm.expect(jsonData.amount).to.eql(500);
  },
);

pm.test(
  "[POST]::/payments/:id - Content check if value for 'net_amount' equal 500",
  function () {
    pm.expect(jsonData.net_amount).to.eql(500);
  },
);

pm.test(
  "[POST]::/payments/:id - Content check if value for 'amount_capturable' equal 0",
  function () {
    pm.expect(jsonData.amount_capturable).to.eql(0);
  },
);

pm.test(
  "[POST]::/payments/:id - Content check if value for 'amount_received' equal 500",
  function () {
    pm.expect(jsonData.amount_received).to.eql(500);
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