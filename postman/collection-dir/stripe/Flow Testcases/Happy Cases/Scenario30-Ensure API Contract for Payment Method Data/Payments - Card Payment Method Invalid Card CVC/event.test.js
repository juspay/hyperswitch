// Validate status 400
pm.test("[POST]::/payments - Status code is 400", function () {
  pm.response.to.be.error
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

pm.test("[POST]::/payments - Response has appropriate error message", function () {
  pm.expect(jsonData.error.message).eql("Invalid card_cvc length");
})
