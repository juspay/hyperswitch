// Validate status 2xx
pm.test("[POST]::/payments - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response has JSON Body
pm.test("[POST]::/payments - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set order_id as variable for jsonData.order_id
if (jsonData?.order_id) {
  pm.collectionVariables.set("order_id", jsonData.order_id);
  console.log(
    "- use {{order_id}} as collection variable for value",
    jsonData.order_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{order_id}}, as jsonData.payment_id is undefined.",
  );
}

// Response body should have value "CHARGED" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'CHARGED'",
    function () {
      pm.expect(jsonData.status).to.eql("CHARGED");
    },
  );
}

// Check if bank_error_code and bank_error_message are empty strings
pm.test("Bank error code is an empty string", function () {
    pm.expect(jsonData.bank_error_code).to.eql("");
});

pm.test("Bank error message is an empty string", function () {
    pm.expect(jsonData.bank_error_message).to.eql("");
});
