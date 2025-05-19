// Validate status 2xx
pm.test("[POST]::/payments/:id/confirm - Status code is 2xx", function () {
  pm.response.to.be.success;
});
