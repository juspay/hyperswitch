// Validate status 4xx
pm.test("[POST]::/refunds - Status code is 4xx", function () {
  pm.response.to.have.status(404);
});

// Validate if response header has matching content-type
pm.test("[GET]::/refunds/:id - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// Response body should have value "pending" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/refunds - Content check if value for 'status' matches 'pending'",
    function () {
      pm.expect(jsonData.status).to.eql("pending");
    },
  );
}

// Response body should have value "540" for "amount"
if (jsonData?.status && pm.collectionVariables.get("refund_id") !== null) {
  pm.test(
    "[POST]::/refunds - Content check if value for 'amount' matches '540'",
    function () {
      pm.expect(jsonData.amount).to.eql(540);
    },
  );
} else {
  pm.test(
    "[POST]::/refunds - Content check if value for 'error.message' matches 'Refund does not exist in our records.'",
    function () {
      pm.expect(jsonData.error.message).to.eql("Refund does not exist in our records.");
    },
  );
}
