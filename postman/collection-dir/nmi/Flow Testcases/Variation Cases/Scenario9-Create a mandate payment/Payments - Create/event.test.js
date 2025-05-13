// Validate status 4xx
pm.test("[POST]::/payments - Status code is 4xx", function () {
  pm.response.to.be.error;
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

// Response body should have value "invalid_request" for "error type"
if (jsonData?.error?.type) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'error.type' matches 'invalid_request'" ,
    function () {
      pm.expect(jsonData.error.type).to.eql("invalid_request");
    },
  );
}

// Response body should have value "mandate payment is not supported by nmi" for "error reason"
if (jsonData?.error?.message) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'error.reason' matches 'credit mandate payment is not supported by nmi'" ,
    function () {
      pm.expect(jsonData.error.reason).to.eql("credit mandate payment is not supported by nmi");
    },
  );
}