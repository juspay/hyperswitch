// Validate status 2xx 
pm.test("[GET]::/payment_methods/:merchant_id - Status code is 2xx", function () {
   pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payment_methods/:merchant_id - Content-Type is application/json", function () {
   pm.expect(pm.response.headers.get("Content-Type")).to.include("application/json");
});


// Parse the response body as JSON
var responseBody = pm.response.json();

// Check if "payment_methods" array contains a "payment_method" with the value "card"
pm.test("[GET]::/payment_methods/:merchant_id  -Content Check if payment_method matches 'card'", function () {
    var paymentMethods = responseBody.payment_methods;
    var cardPaymentMethod = paymentMethods.find(function (method) {
        return method.payment_method == "card";
    });
});

// Check if "payment_methods" array contains a "payment_method" with the value "ideal"
pm.test("[GET]::/payment_methods/:merchant_id - Content Check if payment_method matches 'ideal'", function () {
    var paymentMethods = responseBody.payment_methods;
    var cardPaymentMethod = paymentMethods.find(function (method) {
        return method.payment_method == "ideal";
    });
});

// Check if "payment_methods" array contains a "payment_method" with the value "bank_redirect"
pm.test("[GET]::/payment_methods/:merchant_id  -Content Check if payment_method matches 'bank_redirect'", function () {
    var paymentMethods = responseBody.payment_methods;
    var cardPaymentMethod = paymentMethods.find(function (method) {
        return method.payment_method == "bank_redirect";
    });
});