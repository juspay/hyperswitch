// Validate status 400
pm.test("[POST]::/payments - Status code is 200", function () {
  pm.response.to.be.success
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
} catch (e) { }

// Response body should have value "succeeded" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'status' matches 'succeeded'",
    function () {
      pm.expect(jsonData.status).to.eql("succeeded");
    },
  );
}

pm.test("[POST]::/payments - Response has payment checks", function () {
  pm.expect(jsonData.payment_method_data.card.payment_checks).to.eql({
    "address_line1_check": "pass",
    "address_postal_code_check": "pass",
    "cvc_check": "pass"
  });
})
