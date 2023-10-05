// Validate status 2xx
pm.test(
  "[POST]::/payment_methods/:id/detach - Status code is 2xx",
  function () {
    pm.response.to.be.success;
  },
);

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payment_methods/:id/detach - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);
