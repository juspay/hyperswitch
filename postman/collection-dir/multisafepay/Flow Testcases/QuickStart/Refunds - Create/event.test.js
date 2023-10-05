// Validate status 2xx
pm.test("[POST]::/refunds - Status code is 2xx", function () {
  pm.response.to.be.error;
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
  console.log(
    "INFO - Unable to assign variable {{refund_id}}, as jsonData.refund_id is undefined.",
  );
}

// Response body should have an error message as we try to refund a "processing" payment
if (jsonData?.error) {
  pm.test(
    "[POST]::/payments - Content check if error type is 'invalid_request'",
    function () {
      pm.expect(jsonData.error.message).to.eql(
        "The payment has not succeeded yet. Please pass a successful payment to initiate refund",
      );
    },
  );
}
