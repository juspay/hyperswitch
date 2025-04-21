# Test Plan for MCP Server Client SDK

This document outlines the testing strategy and specific tests for the `mcp_server_client` Python SDK.

## Pending Tests

### Authentication Flow (`test_auth_flow.py`)

-   **Fix Missing Fixture:** Implement the `test_client` fixture required by `test_auth_flow`. This likely involves setting up a test instance of the FastAPI application or using `requests_mock` more extensively.
-   **Happy Path:** Test the complete authentication flow (`sign_in`, `terminate_2fa`) with valid credentials and TOTP token, mocking the external API calls. Verify that a valid JWT is returned.
-   **Invalid Credentials:** Test the `sign_in` function with incorrect email/password. Verify appropriate error handling.
-   **Invalid TOTP Token:** Test the `terminate_2fa` function with an invalid `totp_token`. Verify appropriate error handling.
-   **Expired TOTP Token:** (If applicable) Test the flow with an expired `totp_token`.
-   **Network Errors:** Simulate network errors during API calls and verify graceful handling.

### Business Profile Flow (`test_business_profile_flow.py` - *To be created*)

-   **Create Fixture:** Create a `test_client` fixture (potentially reusing the one from `test_auth_flow.py`).
-   **Happy Path (CRUD):**
    -   Test creating a new business profile (`create_business_profile`). Verify the response.
    -   Test listing business profiles (`list_business_profiles_v1`) and confirm the created profile is present.
    -   Test retrieving the specific business profile (`get_business_profile`). Verify details.
    -   Test updating the business profile (`update_business_profile`) with new data. Verify the update.
    -   Test deleting the business profile (`delete_business_profile`). Verify successful deletion.
    -   Test listing again to confirm deletion.
-   **Error Handling:**
    -   Test creating a profile with invalid data (e.g., missing required fields).
    -   Test retrieving/updating/deleting a non-existent profile ID.
    -   Test operations with an invalid/expired JWT token.
    -   Test operations with an invalid API key.
-   **Network Errors:** Simulate network errors during API calls and verify graceful handling.

### User Management Flow (`test_user_management_flow.py` - *To be created*)

-   **Create Fixture:** Create a `test_client` fixture.
-   **Happy Path:**
    -   Test getting user info (`get_user_info`) with a valid JWT. Verify response.
    -   Test updating user profile (`update_user_profile`) with new name/phone. Verify update.
    -   Test changing password (`change_user_password`) with correct current password.
    -   Test initiating password reset (`initiate_password_reset`). (Requires mocking email sending or checking logs/state).
    -   Test confirming password reset (`confirm_password_reset`) with a valid token. (Requires mocking token generation/validation).
    -   Test verifying email (`verify_user_email`) with a valid token. (Requires mocking token generation/validation).
    -   Test signing out (`sign_out_from_hyperswitch`). Verify token invalidation (may require checking subsequent calls).
-   **Error Handling:**
    -   Test operations with invalid/expired JWT.
    -   Test changing password with incorrect current password.
    -   Test confirming password reset/verifying email with invalid/expired tokens.
-   **Network Errors:** Simulate network errors during API calls and verify graceful handling.

## Test Setup

-   **Mocking:** Use `requests-mock` to simulate responses from the backend API (`localhost:8080`).
-   **Fixtures:** Use `pytest` fixtures to set up common resources like the test client and mock data.
-   **Environment:** Ensure tests run independently and do not rely on a live backend server.

## Running Tests

```bash
cd python-client-sdk/mcp-server
pytest
``` 