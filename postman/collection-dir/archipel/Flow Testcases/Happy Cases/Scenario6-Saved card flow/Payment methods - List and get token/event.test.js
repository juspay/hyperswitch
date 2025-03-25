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

// Response body should have at least one card payment mehod"
pm.test(
"[GET]::/payment_methods/:customer_id  - Content check body has at least one customer_payment_methods",
function () {
    pm.expect(jsonData.customer_payment_methods.length).greaterThan(0);
  },
);

if (jsonData?.customer_payment_methods[0]?.payment_token) {
   pm.collectionVariables.set("payment_token", jsonData.customer_payment_methods[0].payment_token);
   console.log("- use {{payment_token}} as collection variable for value", jsonData.customer_payment_methods[0].payment_token);
} else {
   console.log('INFO - Unable to assign variable {{payment_token}}, as jsonData.customer_payment_methods[0].payment_token is undefined.');
}