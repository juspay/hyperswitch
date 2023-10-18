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

// Verify if the wallet_name in the response matches 'apple_pay'
pm.test(
  "[POST]::/payments/session_tokens - Verify wallet_name is 'apple_pay'",
  function () {
    pm.expect(responseJson.session_token[0].wallet_name).to.eql("apple_pay");
  },
);

// Verify if the wallet_name in the response matches 'google_pay'
pm.test(
  "[POST]::/payments/session_tokens - Verify wallet_name is 'google_pay'",
  function () {
    pm.expect(responseJson.session_token[1].wallet_name).to.eql("google_pay");
  },
);
