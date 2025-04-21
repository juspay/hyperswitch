# Business Profiles API Troubleshooting Guide

This guide helps troubleshoot common issues when working with the Business Profiles API on the localhost:8080 endpoint.

## Common Issues

### 1. Cannot List Business Profiles (404 Error)

**Symptom**: `GET /account/{account_id}/business_profile` returns a 404 error.

**Possible Causes**:
- Incorrect account ID
- Incorrect endpoint path
- API not available on the server

**Solutions**:
1. Verify the account ID is correct
2. Double-check the API path: it should be exactly `/account/{account_id}/business_profile`
3. Confirm the API server is running at localhost:8080: `curl http://localhost:8080/health`
4. Verify authentication credentials (both JWT token and API key)

### 2. Permission Denied Errors (403)

**Symptom**: API calls return 403 Forbidden errors.

**Possible Causes**:
- Missing or invalid JWT token
- Missing or invalid API key
- User doesn't have permissions for the requested operation
- Token has expired

**Solutions**:
1. Verify the JWT token is valid and properly formatted
2. Ensure the API key is correctly passed in the `api-key` header
3. Check that the account ID matches the authenticated user's account
4. Re-authenticate to get a fresh JWT token

### 3. Profile Creation Failures

**Symptom**: `POST /account/{account_id}/business_profile` requests fail.

**Possible Causes**:
- Missing required fields in request body
- Invalid data formats
- Duplicate profile name

**Solutions**:
1. Ensure `profile_name` is provided and not empty
2. Keep profile name under 64 characters
3. Verify all URLs are properly formatted
4. Check for validation error messages in the response

### 4. Cannot Modify or Delete Profiles

**Symptom**: Update or delete operations fail with 403 or 400 errors.

**Possible Causes**:
- Insufficient permissions
- Profile doesn't exist
- Profile ID is incorrect
- Default profile (cannot be deleted)

**Solutions**:
1. Verify you have admin permissions for the account
2. Check if the profile exists using the GET endpoint
3. Confirm you're not trying to delete a default profile
4. Use the correct profile ID from the list operation

## Authentication Issues

### 1. JWT Token Issues

**Symptom**: Authentication errors with message about invalid or expired token.

**Solutions**:
1. Get a fresh JWT token through the authentication flow:
   ```
   POST http://localhost:8080/user/signin
   GET http://localhost:8080/user/2fa/terminate
   ```
2. Make sure the token is properly formatted in the Authorization header:
   ```
   Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
   ```
3. Check for token expiration using a JWT debugger

### 2. API Key Issues

**Symptom**: API calls fail with authentication or permission errors.

**Solutions**:
1. Verify you're including the API key in the `api-key` header
2. Make sure the API key belongs to the account you're trying to access
3. Check that the API key has not been revoked
4. Create a new API key if necessary

## Testing Connectivity

### 1. Verifying API Server

**Command**:
```bash
curl http://localhost:8080/health
```

**Expected Response**:
```json
{"status":"ok"}
```

### 2. Testing List Profiles Endpoint

**Command**:
```bash
curl -X GET \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "api-key: YOUR_API_KEY" \
  http://localhost:8080/account/YOUR_ACCOUNT_ID/business_profile
```

## Connection and Network Issues

### 1. Cannot Connect to localhost:8080

**Symptom**: Requests fail with connection refused or timeout errors.

**Possible Causes**:
- API server is not running
- Port 8080 is not open
- Firewall is blocking requests

**Solutions**:
1. Check if the API server is running:
   ```bash
   ps aux | grep hyperswitch
   ```
2. Verify port 8080 is listening:
   ```bash
   netstat -tuln | grep 8080
   ```
3. Test a simple connection:
   ```bash
   curl http://localhost:8080/health
   ```

### 2. Slow Responses

**Symptom**: API calls take a long time to complete.

**Solutions**:
1. Check system resources (CPU, memory, disk)
2. Check network connectivity and latency
3. Verify the API server is not under high load
4. Use request timeouts in your API calls to fail fast

## Debugging Tools

### 1. Using the Test Script

Our test script can help diagnose issues:

```bash
python test_business_profiles.py --token YOUR_JWT_TOKEN --api_key YOUR_API_KEY --account_id YOUR_ACCOUNT_ID --test list
```

### 2. Using Curl for Direct API Testing

Test the API directly:

```bash
# List profiles
curl -v -X GET \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "api-key: YOUR_API_KEY" \
  -H "Accept: application/json" \
  http://localhost:8080/account/YOUR_ACCOUNT_ID/business_profile

# Create profile
curl -v -X POST \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "api-key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"profile_name": "Test Profile", "description": "Test Description"}' \
  http://localhost:8080/account/YOUR_ACCOUNT_ID/business_profile
```

### 3. Inspecting Request/Response with API Debugger

Enable detailed logging in your application to capture full request and response details.

## Response Status Codes

| Status Code | Description | Common Causes |
|-------------|-------------|--------------|
| 200 | OK | Successful request |
| 201 | Created | Profile successfully created |
| 400 | Bad Request | Invalid input, missing required fields |
| 401 | Unauthorized | Missing or invalid JWT token |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Incorrect endpoint or profile not found |
| 409 | Conflict | Profile name already exists |
| 500 | Internal Server Error | Server-side error |

## Getting Further Help

If you're still experiencing issues after following this guide:

1. Check the logs at `business_profiles_test.log`
2. Verify that you're using the localhost:8080 endpoint exclusively
3. Try the authentication flow again to get fresh tokens
4. Contact the development team with:
   - Full error details
   - Request data that caused the error
   - Steps to reproduce the issue 