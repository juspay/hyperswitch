// Validate status 4xx
pm.test("[POST]::/user/v2/signin?token_only=true - Status code is 401", function () {
  pm.response.to.have.status(401);
});

// Validate if response header has matching content-type
pm.test("[POST]::user/v2/signin?token_only=true - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[POST]::user/v2/signin?token_only=true - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});