console.log("X-Request-ID:", pm.response.headers.get("X-Request-ID"));

// Validate status 2xx
pm.test("[POST]::user/v2/signin?token_only=true - Status code is 2xx", function () {
  pm.response.to.be.success;
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

// Validate specific JSON response content
pm.test("[POST]::user/v2/signin?token_only=true - Response contains token", function () {
  var jsonData = pm.response.json();
  pm.expect(jsonData).to.have.property("token");
  pm.expect(jsonData.token).to.be.a("string").and.to.not.be.empty;
});