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

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// Validate if response has JSON Body
pm.test("[GET]::/payments/:id - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

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

// Response body should have value "cancellation succeeded" for "payment status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments/:id/cancel - Content check if value for 'jsonData.status' matches 'partially_captured_and_capturable'",
    function () {
      pm.expect(jsonData.status).to.eql("partially_captured_and_capturable");
    },
  );
}
