
var baseEmail = pm.environment.get('user_base_email_for_signup');
var emailDomain = pm.environment.get("user_domain_for_signup");

// Generate a unique email address
var uniqueEmail = baseEmail + new Date().getTime() + emailDomain;
// Set the unique email address as an environment variable
pm.environment.set('unique_email', uniqueEmail);
