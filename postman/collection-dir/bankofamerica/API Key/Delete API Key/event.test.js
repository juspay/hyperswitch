// Validate status 2xx
pm.test(
  "[DELETE]::/api_keys/:merchant_id/:api-key - Status code is 2xx",
  function () {
    pm.response.to.be.success;
  },
);

// Validate if response header has matching content-type
pm.test(
  "[DELETE]::/api_keys/:merchant_id/:api-key - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);
