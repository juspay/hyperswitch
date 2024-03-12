// Validate status 2xx 
pm.test("[POST]::/payouts/create - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payouts/create - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[POST]::/payouts/create - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});


// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// Response body should have value "true" for "value"
if (jsonData?.value) {
  pm.test(
    "[POST]::/configs - Content check if value for 'value' matches 'true'",
    function () {
      pm.expect(jsonData.value).to.eql("true");
    },
  );
}