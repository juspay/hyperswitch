// Validate status 2xx 
pm.test("[POST]::/account/:account_id/connectors - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/account/:account_id/connectors - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// pm.collectionVariables - Set mca_wise_id as variable for jsonData.merchant_connector_id
if (jsonData?.merchant_connector_id) {
   pm.collectionVariables.set("mca_wise_id", jsonData.merchant_connector_id);
   console.log("- use {{mca_wise_id}} as collection variable for value",jsonData.merchant_connector_id);
} else {
   console.log('INFO - Unable to assign variable {{mca_wise_id}}, as jsonData.merchant_connector_id is undefined.');
};

// pm.collectionVariables - Set profile_id as variable for jsonData.profile_id
if (jsonData?.profile_id) {
   pm.collectionVariables.set("profile_id", jsonData.profile_id);
   console.log("- use {{profile_id}} as collection variable for value",jsonData.profile_id);
} else {
   console.log('INFO - Unable to assign variable {{profile_id}}, as jsonData.profile_id is undefined.');
};
