// Validate status 2xx 
pm.test("[GET]::/payment_methods/:customer_id - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payment_methods/:customer_id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

pm.test("[GET]::/payment_methods/:customer_id Check if card not stored in the locker ", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.customer_payment_methods).to.be.an('array').that.is.empty;
});