// Validate status 2xx
pm.test("[POST]::/health - Status code is 2xx", function () {
  pm.response.to.be.success;
});
