// Validate status 2xx
pm.test("[GET]::/refunds/:id - Status code is 2xx", function () {
  pm.response.to.be.success;
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

// pm.collectionVariables - Set refund_id as variable for jsonData.payment_id
if (jsonData?.refund_id) {
  pm.collectionVariables.set("refund_id", jsonData.refund_id);
  console.log(
    "- use {{refund_id}} as collection variable for value",
    jsonData.refund_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{refund_id}}, as jsonData.refund_id is undefined.",
  );
}

// Response body should have value "succeeded" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/refunds - Content check if value for 'status' matches 'pending'",
    function () {
      pm.expect(jsonData.status).to.eql("pending");
    },
  );
}

// Response body should have value "6540" for "amount"
if (jsonData?.status) {
  pm.test(
    "[POST]::/refunds - Content check if value for 'amount' matches '1000'",
    function () {
      pm.expect(jsonData.amount).to.eql(1000);
    },
  );
}
