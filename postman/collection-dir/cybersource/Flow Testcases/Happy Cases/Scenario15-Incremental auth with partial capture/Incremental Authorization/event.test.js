pm.test("[POST]::/payments:id/incremental_authorizations - Content check if value for 'amount' matches '1001'", function () {
    // Parse the response JSON
    var jsonData = pm.response.json();

    // Check if the 'amount' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations[0].amount).to.eql(1001);
});

pm.test("[POST]::/payments:id/incremental_authorizations - Content check if value for 'status' matches 'success'", function () {
    // Parse the response JSON
    var jsonData = pm.response.json();

    // Check if the 'status' in the response matches the expected value
    pm.expect(jsonData.incremental_authorizations[0].status).to.eql("success");
});
