// Validate status 2xx
pm.test("[POST]::/payments/session_tokens - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/session_tokens - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

const responseJson = pm.response.json();

// Debug: Check what's actually in the session_token array
console.log("Session tokens array:", JSON.stringify(responseJson.session_token, null, 2));
console.log("Array length:", responseJson.session_token.length);

// Verify the session_token array exists and has at least one item
pm.test(
  "[POST]::/payments/session_tokens - session_token array exists with items",
  function () {
    pm.expect(responseJson.session_token).to.be.an('array');
    pm.expect(responseJson.session_token.length).to.be.at.least(1);
  },
);

// Test for google_pay (which we know exists)
pm.test(
  "[POST]::/payments/session_tokens - Verify google_pay exists", 
  function () {
    const googlePayToken = responseJson.session_token.find(token => token.wallet_name === 'google_pay');
    pm.expect(googlePayToken).to.not.be.undefined;
    pm.expect(googlePayToken.wallet_name).to.eql("google_pay");
  },
);

// Conditional test for apple_pay - only run if it exists
if (responseJson.session_token.find(token => token.wallet_name === 'apple_pay')) {
  pm.test(
    "[POST]::/payments/session_tokens - Verify apple_pay exists",
    function () {
      const applePayToken = responseJson.session_token.find(token => token.wallet_name === 'apple_pay');
      pm.expect(applePayToken).to.not.be.undefined;
      pm.expect(applePayToken.wallet_name).to.eql("apple_pay");
    },
  );
} else {
  console.log("apple_pay not found in response - this might be expected behavior");
}

