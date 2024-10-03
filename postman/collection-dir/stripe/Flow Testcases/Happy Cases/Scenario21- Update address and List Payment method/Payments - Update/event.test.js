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

// Check if the 'currency' is equal to "EUR"
pm.test("[POST]::/payments/:id -Content Check if 'currency' matches 'EUR' ", function () {
    pm.expect(jsonData.currency).to.eql("EUR");
});

// Extract the "country" field from the JSON data
var country = jsonData.billing.address.country;

// Check if the country is "NL"
pm.test("[POST]::/payments/:id -Content Check if billing 'Country' matches NL (Netherlands)", function () {
    pm.expect(country).to.equal("NL");
});

var country1 = jsonData.shipping.address.country;

// Check if the country is "NL"
pm.test("[POST]::/payments/:id -Content Check if shipping 'Country' matches NL (Netherlands)", function () {
    pm.expect(country1).to.equal("NL");
});
