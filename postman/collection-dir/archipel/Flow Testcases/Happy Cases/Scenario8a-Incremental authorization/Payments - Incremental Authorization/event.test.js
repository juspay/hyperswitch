
// Validate status 2xx
pm.test("[POST]::/payments/:id/capture - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/capture - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);
// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/capture - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

var jsonData = pm.response.json();

pm.test("[POST]::/payments:id/incremental_authorizations - Content check have at least one incremental_authorizations ", function () {
    // Check if the 'amount' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations.length).greaterThan(0);
});

pm.test("[POST]::/payments:id/incremental_authorizations - Content check if value for 'amount' matches '1001'", function () {
    // Check if the 'amount' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations[0].amount).to.eql(1001);
});

pm.test("[POST]::/payments:id/incremental_authorizations - Content check if value for 'previously_authorized_amount' matches '500'", function () {
    // Check if the 'amount' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations[0].previously_authorized_amount).to.eql(500);
});

pm.test("[POST]::/payments:id/incremental_authorizations - Content check if value for 'status' matches 'success'", function () {
    // Parse the response JSON
    var jsonData = pm.response.json();

    // Check if the 'status' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations[0].status).to.eql("success");
});