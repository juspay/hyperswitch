// Validate status 2xx 
pm.test("[GET]::/payment_methods/:customer_id - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payment_methods/:customer_id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Validate if response header has matching content-type
pm.test("[GET]::/payment_methods/:customer_id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});

// Set response object as internal variable
let jsonData = {};
try {jsonData = pm.response.json();}catch(e){}

// pm.collectionVariables - Set card_payout_token as variable for jsonData.customer_payment_methods[2].payment_token
if (jsonData?.customer_payment_methods[2]?.payment_token) {
   pm.collectionVariables.set("card_payout_token", jsonData.customer_payment_methods[2].payment_token);
   console.log("- use {{card_payout_token}} as collection variable for value",jsonData.customer_payment_methods[2].payment_token);
} else {
   console.log('INFO - Unable to assign variable {{card_payout_token}}, as jsonData.customer_payment_methods[2].payment_token is undefined.');
};

// pm.collectionVariables - Set sepa_payout_token as variable for jsonData.customer_payment_methods[1].payment_token
if (jsonData?.customer_payment_methods[1]?.payment_token) {
   pm.collectionVariables.set("sepa_payout_token", jsonData.customer_payment_methods[1].payment_token);
   console.log("- use {{sepa_payout_token}} as collection variable for value",jsonData.customer_payment_methods[1].payment_token);
} else {
   console.log('INFO - Unable to assign variable {{sepa_payout_token}}, as jsonData.customer_payment_methods[1].payment_token is undefined.');
};

// pm.collectionVariables - Set bacs_payout_token as variable for jsonData.customer_payment_methods[0].payment_token
if (jsonData?.customer_payment_methods[0]?.payment_token) {
   pm.collectionVariables.set("bacs_payout_token", jsonData.customer_payment_methods[0].payment_token);
   console.log("- use {{bacs_payout_token}} as collection variable for value",jsonData.customer_payment_methods[0].payment_token);
} else {
   console.log('INFO - Unable to assign variable {{bacs_payout_token}}, as jsonData.customer_payment_methods[0].payment_token is undefined.');
};
