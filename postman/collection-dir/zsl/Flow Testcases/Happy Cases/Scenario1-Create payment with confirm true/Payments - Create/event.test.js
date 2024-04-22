// Validate status 2xx
pm.test("[POST]::/payments - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payments - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
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

// Response body should have redirect_to_url as next action type
if (jsonData?.next_action.type) {
  pm.test(
    "[POST]::/payments:id/confirm - Next Action Check",
    function () {
      pm.expect(jsonData.next_action.type).to.eql("redirect_to_url");
    },
  );
}

// Response body should have status = requires_customer_action
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments:id/confirm - Next Action Check",
    function () {
      pm.expect(jsonData.status).to.eql("requires_customer_action");
    },
  );
}


