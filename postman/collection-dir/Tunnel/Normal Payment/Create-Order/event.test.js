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

// Response body should have value "CREATED" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'CREATED'",
    function () {
      pm.expect(jsonData.status).to.eql("CREATED");
    },
  );
}

// Response body should contain order_id
if (jsonData?.order_id) {
  pm.test(
    "[POST]::/payments - Content check if 'order_id' is present and not empty",
    function () {
      pm.expect(jsonData.order_id).to.be.a("string").and.not.empty;
    },
  );
} else {
  pm.test(
    "[POST]::/payments - Response body does not contain 'order_id' field",
    function () {
      pm.expect.fail("The 'order_id' field was not found in the response body.");
    },
  );
}



