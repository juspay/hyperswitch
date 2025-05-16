// Validate status 2xx
pm.test("[POST]::/payments/:id/confirm - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/confirm - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/confirm - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) { }

// Response body should have value "requires_capture" for "status"
pm.test(
  "[POST]::/payments - Content check if value for 'status' matches 'requires_capture'",
  function () {
    pm.expect(jsonData.status).to.eql("requires_capture");
  },
);

pm.test(
  "[POST]::/payments - Content check if 'connector_transaction_id' exists",
  function () {
    pm.expect(typeof jsonData.connector_transaction_id !== "undefined").to.be
      .true;
    pm.collectionVariables.set("connector_transaction_id", jsonData.connector_transaction_id)
  },
);

pm.test(
  "[POST]::/payments - Content check if 'connector' is archipel",
  function () {
    pm.expect(jsonData.connector).to.be.equal("archipel");
  },
);
