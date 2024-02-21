// Validate status 2xx
pm.test("[DELETE]::/customers/:id - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[DELETE]::/customers/:id - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[DELETE]::/customers/:id - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});
