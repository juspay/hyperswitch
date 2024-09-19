// Validate status 4xx
pm.test("[GET]::/payments/:id - Status code is 4xx", function () {
  pm.response.to.be.error;
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

// Response body should have value "connector error" for "error type"
if (jsonData?.error?.type) {
  pm.test(
    "[POST]::/payments - Content check if value for 'error.type' matches 'invalid_request'",
    function () {
      pm.expect(jsonData.error.type).to.eql("invalid_request");
    },
  );
}

pm.test("[POST]::/payments - Content check if value for 'error.reason' matches 'Mandate has already been revoked", function () {
    pm.expect(jsonData.error.reason).to.eql("Mandate has already been revoked");
});
