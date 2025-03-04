// Validate status 4xx
pm.test("[POST]::/refunds - Status code is 4xx", function () {
  pm.response.to.have.status(400);
});

// Validate if response header has matching content-type
pm.test("[POST]::/refunds - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set refund_id as variable for jsonData.payment_id
if (jsonData?.refund_id) {
  pm.collectionVariables.set("refund_id", jsonData.refund_id);
  console.log(
    "- use {{refund_id}} as collection variable for value",
    jsonData.refund_id,
  );
} else {
  pm.collectionVariables.set("refund_id", null);
  pm.test(
    "[POST]::/refunds - Content check if 'error.message' matches 'This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured'",
    function () {
      pm.expect(jsonData.error.message).to.eql("This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured");
    },
  );
  console.log(
    "INFO - Unable to assign variable {{refund_id}}, as jsonData.refund_id is undefined.",
  );
}

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
if (jsonData?.status) {
  pm.test(
    "[POST]::/refunds - Content check if value for 'amount' matches '540'",
    function () {
      pm.expect(jsonData.amount).to.eql(540);
    },
  );
}
