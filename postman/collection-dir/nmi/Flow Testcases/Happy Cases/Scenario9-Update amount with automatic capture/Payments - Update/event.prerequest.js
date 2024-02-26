// Get the value of 'amount' from the environment
const updated_amount = _.random(100, 100000);
pm.environment.set("another_random_number", updated_amount);
