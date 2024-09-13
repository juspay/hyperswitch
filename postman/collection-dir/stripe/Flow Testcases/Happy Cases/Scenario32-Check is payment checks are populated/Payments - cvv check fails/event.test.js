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

pm.test("[POST]::/payments - Response has cvc check as fail", function () {
  pm.expect(jsonData.payment_method_data.card.payment_checks.cvc_check).to.eql("fail");
})
