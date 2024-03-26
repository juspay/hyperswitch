// Get the value of 'amount' from the environment
const amount = pm.environment.get("amount");

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

//// Response body should have value "processing" for "status"
if (jsonData?.status) {
pm.test("[POST]::/payments - Content check if value for 'status' matches 'processing'", function() {
  pm.expect(jsonData.status).to.eql("processing");
})};


// Check if the 'amount' is equal to "amount"
pm.test("[POST]::/payments/:id -Content Check if 'amount' matches '{{amount}}' ", function () {
    pm.expect(jsonData.amount).to.eql(amount);
});

//// Response body should have value "amount_received" for "amount"
if (jsonData?.amount_received) {
pm.test("[POST]::/payments - Content check if value for 'amount_received' matches '{{amount}}'", function() {
  pm.expect(jsonData.amount_received).to.eql(amount);
})};
