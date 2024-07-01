console.log("[LOG]::x-request-id - " + pm.response.headers.get("x-request-id"));

// Validate status 2xx
pm.test("[POST]::/user/connect_account - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/user/connect_account - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[POST]::/user/connect_account - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Validate specific JSON response content
pm.test("[POST]::/user/connect_account - Response contains is_email_sent", function () {
  var jsonData = pm.response.json();
  pm.expect(jsonData).to.have.property("is_email_sent");
  pm.expect(jsonData.is_email_sent).to.be.true;
});

