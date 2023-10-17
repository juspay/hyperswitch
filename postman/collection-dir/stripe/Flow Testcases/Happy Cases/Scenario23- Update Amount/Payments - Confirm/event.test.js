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

//// Response body should have value "succeeded" for "status"
if (jsonData?.status) {
pm.test("[POST]::/payments - Content check if value for 'status' matches 'succeeded'", function() {
  pm.expect(jsonData.status).to.eql("succeeded");
})};


// Check if the 'amount' is equal to "1000"
pm.test("[POST]::/payments/:id -Content Check if 'amount' matches '1000' ", function () {
    pm.expect(jsonData.amount).to.eql(1000);
});

//// Response body should have value "amount_received" for "1000"
if (jsonData?.amount_received) {
pm.test("[POST]::/payments - Content check if value for 'amount_received' matches '1000'", function() {
  pm.expect(jsonData.amount_received).to.eql(1000);
})};
