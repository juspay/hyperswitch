# Business Profiles Management - Execution Document

## Implementation Status

*Note: The initial plan referred to `/business_profiles` endpoints. The actual implemented and working endpoints use the `/account/{account_id}/business_profile` path.*

| API Endpoint | Method | Status | Notes |
|--------------|--------|--------|-------|
| `/account/{account_id}/business_profile` | GET | ✅ Implemented & Verified | Lists profiles for a merchant account. Verified 2024-08-20. |
| `/account/{account_id}/business_profile/{profile_id}` | GET | ✅ Implemented & Verified | Retrieves a specific profile. Verified 2024-08-20. |
| `/account/{account_id}/business_profile` | POST | ✅ Implemented & Fixed | Creates a profile. Fixed payload issues 2024-08-20. |
| `/account/{account_id}/business_profile/{profile_id}` | POST | ✅ Implemented & Fixed | Updates a profile (Note: uses POST). Fixed payload/permission issues 2024-08-20. |
| `/account/{account_id}/business_profile/{profile_id}` | DELETE | ✅ Implemented & Verified | Deletes a profile. Verified via refactoring 2024-08-20. |

## Business Profiles Management Implementation Details

The following functionalities, corresponding to the `/account/{account_id}/business_profile` endpoints, have been implemented and verified:

- **List Profiles:** Retrieves all profiles for the specified merchant account.
- **Get Profile:** Retrieves details for a single profile ID.
- **Create Profile:** Creates a new profile. Required debugging based on test scripts to identify correct payload structure (including required boolean/integer fields and correct handling of metadata/description).
- **Update Profile:** Updates an existing profile. Required debugging to confirm POST method and correct payload structure. Also identified potential permission issues depending on API key and target profile.
- **Delete Profile:** Deletes a specific profile.

All implementations utilize the corresponding functions in `business_profiles.py` and are exposed via MCP tools in `server.py`.

## API Endpoint Configuration

All implemented API endpoints use `localhost:8080` as the base URL. The correct, working endpoints are:

- `http://localhost:8080/account/{account_id}/business_profile` - For listing and creating profiles
- `http://localhost:8080/account/{account_id}/business_profile/{profile_id}` - For retrieving, updating (POST), and deleting specific profiles

## Authentication Requirements

- All endpoints require JWT authentication via the `Authorization: Bearer <token>` header.
- An `api-key` header is also required. The specific key needed might vary (e.g., standard key vs. `'hyperswitch'`) depending on the operation and target profile, especially for updates.

## Business Profile Design Decisions

*(This section reflects the implemented structure based on debugging)*

1. **Profile Structure**:
   - Core fields: `profile_name`, `return_url`, `enable_payment_response_hash`, `redirect_to_merchant_with_http_post`, `use_billing_as_payment_method_billing`, `session_expiry`.
   - Nested `webhook_details` object for `webhook_url` and `webhook_version`.
   - `metadata` field allows custom attributes, but structure might be validated by API.
   - `description` appears to be handled within `metadata` rather than as a top-level field during create/update.

2. **API Design**:
   - Mostly RESTful, but Update uses POST instead of PUT/PATCH.
   - Consistent response formats observed.
   - API appears sensitive to exact payload structure for create/update.

3. **Permissions Model**:
   - Read operations (List/Get) seem less restrictive.
   - Write operations (Create/Update/Delete) require appropriate permissions tied potentially to the API key (`hyperswitch` key worked for updating MCP-created profile) and the specific profile being targeted.

## Implementation Plan

*(This plan reflects the work DONE for the `/account/...` endpoints)*

### Phase 1: Core Implementation
- [X] Create the `business_profiles.py` module.
- [X] Implement List and Get endpoints.
- [X] Set up data models (e.g., `BusinessProfile` class).
- [X] Use `localhost:8080` for API calls.

### Phase 2: Complete Implementation
- [X] Implement Create, Update, and Delete endpoints.
- [X] Add error handling and logging.

### Phase 3: Testing and Refinement
- [X] Debug and fix Create/Update based on test scripts and iterative testing.
- [X] Refactor update logic for code consistency.

## Testing Plans

- [X] Utilized `run_business_profiles_test.sh` for debugging Create/Update.
- [X] Performed functional testing via MCP tool calls after fixes.
- Further unit/integration tests could be added.

## Next Steps

*(Based on current status)*

1.  Add more robust unit/integration tests for `business_profiles.py` functions.
2.  Clarify definitive API key requirements for different operations/profiles.
3.  Clarify exact API expectations for `metadata` structure during create/update.
4.  Update `group4_business_profiles/plan.md` to align with this execution log or remove/archive Group 4 docs if `/business_profiles` path is obsolete.

## Open Questions

*(These remain relevant)*

1. Should business profiles support versioning?
2. What pagination strategy should be used for listing profiles?
3. Should we implement bulk operations for profile management?
4. How should we handle profile deletion (soft delete vs. hard delete)?

## Resources

- [Hyperswitch API Documentation](http://localhost:8080/docs) *(May need updating?)*
- [JWT Authentication Flow](../group1_authentication/plan.md)
- [Troubleshooting Guide](../troubleshooting_guide.md)
- `run_business_profiles_test.sh` *(Key debugging resource)* 