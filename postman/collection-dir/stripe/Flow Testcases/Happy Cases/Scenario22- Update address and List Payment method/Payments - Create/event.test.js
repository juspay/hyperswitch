// Validate status 2xx 
pm.test("[POST]::/payments - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payments - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[POST]::/payments - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// pm.collectionVariables - Set payment_id as variable for jsonData.payment_id
if (jsonData?.payment_id) {
   pm.collectionVariables.set("payment_id", jsonData.payment_id);
   console.log("- use {{payment_id}} as collection variable for value",jsonData.payment_id);
} else {
   console.log('INFO - Unable to assign variable {{payment_id}}, as jsonData.payment_id is undefined.');
};


// pm.collectionVariables - Set mandate_id as variable for jsonData.mandate_id
if (jsonData?.mandate_id) {
   pm.collectionVariables.set("mandate_id", jsonData.mandate_id);
   console.log("- use {{mandate_id}} as collection variable for value",jsonData.mandate_id);
} else {
   console.log('INFO - Unable to assign variable {{mandate_id}}, as jsonData.mandate_id is undefined.');
};

// pm.collectionVariables - Set client_secret as variable for jsonData.client_secret
if (jsonData?.client_secret) {
   pm.collectionVariables.set("client_secret", jsonData.client_secret);
   console.log("- use {{client_secret}} as collection variable for value",jsonData.client_secret);
} else {
   console.log('INFO - Unable to assign variable {{client_secret}}, as jsonData.client_secret is undefined.');
};

// pm.collectionVariables - Set customer_id as variable for jsonData.customer_id
if (jsonData?.customer_id) {
   pm.collectionVariables.set("customer_id", jsonData.customer_id);
   console.log("- use {{customer_id}} as collection variable for value",jsonData.customer_id);
} else {
   console.log('INFO - Unable to assign variable {{customer_id}}, as jsonData.customer_id is undefined.');
};
// Response body should have value "requires_payment_method" for "status"
if (jsonData?.status) {
pm.test("[POST]::/payments - Content check if value for 'status' matches 'requires_payment_method'", function() {
  pm.expect(jsonData.status).to.eql("requires_payment_method");
})};
