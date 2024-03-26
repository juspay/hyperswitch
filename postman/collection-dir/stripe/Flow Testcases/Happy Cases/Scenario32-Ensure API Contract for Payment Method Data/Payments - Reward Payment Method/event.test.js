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
} catch (e) { }

// Response body should have value "requires_payment_method" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'requires_confirmation'",
    function () {
      pm.expect(jsonData.status).to.eql("requires_confirmation");
    },
  );
}
