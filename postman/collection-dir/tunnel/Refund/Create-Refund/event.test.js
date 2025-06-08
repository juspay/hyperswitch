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

// Check if the 'refunds' array exists and is not empty
pm.test("Refunds array exists and is not empty", function () {
    pm.expect(jsonData.refunds).to.be.an('array').and.to.have.lengthOf.at.least(1);
});

const refundBody = jsonData.refunds[0];

// Verify refund status
pm.test("Refund status is 'PENDING'", function () {
    pm.expect(refundBody.status).to.eql("PENDING");
});

pm.test("Refund amount is 1", function () {
    pm.expect(refundBody.amount).to.eql(1);
});

