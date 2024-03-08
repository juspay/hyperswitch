// Validate status 2xx 
pm.test("[POST]::/payouts/:id/fulfill - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payouts/:id/fulfill - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[POST]::/payouts/:id/fulfill - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// Validate if payout was successful
pm.test("[POST]::/payouts/:id/fulfill - Payout was successful", function () {
    pm.expect(jsonData.status).eql("success");
});
