// Validate status 2xx
pm.test("[POST]::/accounts - Status code is 2xx", function () {
  pm.response.to.be.success;
});
