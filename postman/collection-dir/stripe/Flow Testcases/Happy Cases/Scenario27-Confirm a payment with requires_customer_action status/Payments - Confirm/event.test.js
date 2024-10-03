// Validate status 2xx
pm.test("[GET]::/payments/:id - Status code is 400", function () {
  pm.response.to.be.error;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payments/:id - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[GET]::/payments/:id - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) { }


// Response body should have appropriatae error message
if (jsonData?.message) {
  pm.test(
    "Content check if appropriate error message is present",
    function () {
      pm.expect(jsonData.message).to.eql("You cannot confirm this payment because it has status requires_customer_action");
    },
  );
}
