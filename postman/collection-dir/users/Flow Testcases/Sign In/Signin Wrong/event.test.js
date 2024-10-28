console.log("[LOG]::x-request-id - " + pm.response.headers.get("x-request-id"));

// Validate status code is 4xx Bad Request
pm.test("[POST]::/user/signin - Status code is 401", function () {
  pm.response.to.have.status(401);
});

// Validate if response header has matching content-type
pm.test("[POST]::/user/signin - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[POST]::/user/signin - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});