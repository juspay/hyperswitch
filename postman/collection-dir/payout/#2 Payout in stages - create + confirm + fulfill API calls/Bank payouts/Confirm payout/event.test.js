// Validate status 2xx 
pm.test("[PUT]::/payouts/:id - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[PUT]::/payouts/:id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[PUT]::/payouts/:id - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// Validate if payout requires fulfillment
pm.test("[PUT]::/payouts/:id - Payout requires fulfillment", function () {
    pm.expect(jsonData.status).eql("requires_fulfillment");
});

// pm.collectionVariables - Set payout_id as variable for jsonData.payout_id
if (jsonData?.payout_id) {
   pm.collectionVariables.set("payout_id", jsonData.payout_id);
   console.log("- use {{payout_id}} as collection variable for value",jsonData.payout_id);
} else {
   console.log('INFO - Unable to assign variable {{payout_id}}, as jsonData.payout_id is undefined.');
};