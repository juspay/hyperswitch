// Validate status 2xx 
pm.test("[POST]::/routing/payouts/:id/activate - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/routing/payouts/:id/activate - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[POST]::/routing/payouts/:id/activate - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch(e) {
}

// Validate if algorithm type is advanced
pm.test("[POST]::/routing/payouts/:id/activate - Algorithm configured for payouts", function () {
    pm.expect(jsonData.kind).to.eql("advanced");
});

// Validate if algorithm was configured for payouts
pm.test("[POST]::/routing/payouts/:id/activate - Algorithm configured for payouts", function () {
    pm.expect(jsonData.algorithm_for).to.eql("payout");
});
