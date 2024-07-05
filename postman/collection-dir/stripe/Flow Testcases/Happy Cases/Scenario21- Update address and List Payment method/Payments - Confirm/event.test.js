// Validate status 2xx 
pm.test("[POST]::/payments/:id/confirm - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payments/:id/confirm - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// Validate if response has JSON Body 
pm.test("[POST]::/payments/:id/confirm - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});

//// Response body should have value "requires_customer_action" for "status"
if (jsonData?.status) {
pm.test("[POST]::/payments - Content check if value for 'status' matches 'requires_customer_action'", function() {
  pm.expect(jsonData.status).to.eql("requires_customer_action");
})};

