# API Keys Management Troubleshooting Guide

This document provides solutions for common issues encountered when working with API Keys in the Hyperswitch platform.

## Common Issues and Solutions

### Authentication Problems

#### Issue: "Unauthorized" error when using API Key
- **Symptoms**: Receiving 401 Unauthorized responses when using an API key
- **Possible Causes**:
  - API key has expired
  - API key has been revoked
  - Using a publishable key for a request that requires a secret key
  - Key does not have required permissions
- **Solutions**:
  - Verify the expiration date of your API key using `GET /api_keys/{key_id}`
  - Create a new API key if yours has expired
  - Ensure you're using the correct key type (secret vs. publishable) for the operation
  - Check that the key has the necessary permissions for the operation

#### Issue: "Invalid API Key" error
- **Symptoms**: Receiving a "Invalid API Key" response
- **Possible Causes**:
  - Typo in the API key
  - Key has been deleted
  - Key format is incorrect
- **Solutions**:
  - Double-check the API key for typos
  - Verify the key exists by listing all API keys with `GET /api_keys`
  - Ensure the key follows the expected format (sk_* for secret keys, pk_* for publishable keys)

### API Key Management Issues

#### Issue: Can't retrieve the full API key after creation
- **Symptoms**: The full API key is not visible in subsequent API calls
- **Cause**: For security reasons, the full API key is only displayed once at creation time
- **Solutions**:
  - Always securely store the full API key when it's first created
  - If the key is lost, you'll need to create a new API key and delete the old one

#### Issue: API key creation fails
- **Symptoms**: Receiving errors when trying to create a new API key
- **Possible Causes**:
  - Maximum number of API keys reached
  - Invalid parameters provided
  - Insufficient permissions
- **Solutions**:
  - List existing keys and delete unused ones if you've reached the limit
  - Check the API documentation for required and optional parameters
  - Ensure your JWT token has sufficient permissions to create API keys

#### Issue: Unable to delete API key
- **Symptoms**: Delete operation returns an error or fails
- **Possible Causes**:
  - Key ID is incorrect
  - You don't have permission to delete the key
  - Key is currently in use by active integrations
- **Solutions**:
  - Verify the key ID by listing all keys first
  - Check your permissions
  - Make sure the key is not actively being used elsewhere before deletion

### API Key Usage Issues

#### Issue: Requests fail with "Rate limit exceeded" error
- **Symptoms**: Receiving 429 Too Many Requests responses
- **Cause**: Exceeding the rate limit associated with your API key
- **Solutions**:
  - Implement proper backoff and retry logic in your code
  - Consider upgrading your account for higher rate limits
  - Optimize your code to reduce the number of API calls

#### Issue: "Access denied" for specific operations
- **Symptoms**: Some API calls work but others fail with access denied errors
- **Cause**: Key doesn't have the necessary permissions for all operations
- **Solutions**:
  - Create different keys with appropriate permissions for different services
  - Update the key's permissions if supported
  - Use a key with broader permissions for operations that require it

### Business Profile Related Issues

#### Issue: API key doesn't work with multiple business profiles
- **Symptoms**: API key works for one business profile but not others
- **Cause**: API keys are scoped to specific business profiles
- **Solutions**:
  - Create separate API keys for each business profile
  - Use the appropriate key for each business profile
  - Include the `profile_id` parameter in requests when required

## Debugging Tips

### Logging API Key Operations

Add the following code to log API key operations for debugging:

```python
import logging

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    filename='api_key_debug.log'
)

# Log before making API key requests
logging.debug(f"Making API key request: {operation}")

# Log after receiving response
logging.debug(f"Response received: {response.status_code}")
```

### Testing API Key Validity

Use this code snippet to test if an API key is valid:

```python
def validate_api_key(api_key):
    """Test if an API key is valid and return its details"""
    try:
        # Make a simple request that requires authentication
        # For example, fetching your account info
        response = requests.get(
            "https://api.hyperswitch.io/v1/account",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json"
            }
        )
        
        if response.status_code == 200:
            print("API key is valid")
            return True
        else:
            print(f"API key validation failed: {response.status_code} - {response.text}")
            return False
    except Exception as e:
        print(f"Error validating API key: {e}")
        return False
```

## Requesting Support

If you've tried the troubleshooting steps and still encounter issues:

1. Generate a detailed error report by running the test script with verbose logging:
   ```
   python test_api_keys.py --verbose
   ```

2. Contact Hyperswitch support with:
   - Your error logs
   - API key ID (never share the full key)
   - Steps to reproduce the issue
   - Any error messages or codes received

## Best Practices to Avoid Issues

1. **Store API keys securely**: Never hardcode keys in your application or commit them to version control.

2. **Implement key rotation**: Regularly rotate API keys to minimize the impact of potential key compromises.

3. **Use environment variables**: Store API keys in environment variables rather than in your code.

4. **Add exponential backoff**: Implement exponential backoff for retry logic to handle rate limiting.

5. **Monitor key usage**: Regularly check key usage patterns to identify unauthorized or unusual activity.

6. **Use the least privilege principle**: Create keys with only the permissions they need for specific operations.

7. **Set appropriate expiration periods**: Choose expiration periods based on security requirements.

8. **Log key creation events**: Keep audit logs of when keys are created and deleted.

## Additional Resources

- [Hyperswitch API Keys Documentation](https://docs.hyperswitch.io/api-reference/api-keys)
- [API Authentication Best Practices](https://docs.hyperswitch.io/api-reference/authentication)
- [Security Guidelines for API Keys](https://docs.hyperswitch.io/security/api-keys) 