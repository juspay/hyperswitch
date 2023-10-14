// Validate status 2xx 
pm.test("[POST]::/payments/:id - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payments/:id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response has JSON Body 
pm.test("[POST]::/payments/:id - Response has JSON Body", function () {
    pm.response.to.have.jsonBody();
});


// Parse the JSON response
var jsonData = pm.response.json();

// Check if the 'amount' is equal to "1000"
pm.test("[POST]::/payments/:id -Content Check if 'amount' matches '1000' ", function () {
    pm.expect(jsonData.amount).to.eql(1000);
});


