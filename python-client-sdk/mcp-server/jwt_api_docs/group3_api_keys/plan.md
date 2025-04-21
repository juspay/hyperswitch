# Group 3: API Keys Management - Implementation Plan

## Overview

This group covers the API Keys Management endpoints that utilize JWT tokens. API keys are essential for integrating with Hyperswitch and enabling secure programmatic access to the platform. These endpoints allow users to create, list, retrieve, update, and delete API keys that can be used for authenticating API requests.

## APIs in this Group

**API Keys Management**
- `/api_keys` - List API keys (GET)
- `/api_keys/{key_id}` - Get API key details (GET)
- `/api_keys` - Create API key (POST)
- `/api_keys/{key_id}` - Update API key (PUT)
- `/api_keys/{key_id}` - Delete API key (DELETE)

## Implementation Steps

### 1. Create API Keys Module (`api_keys.py`)

- [ ] Implement `list_api_keys` function
- [ ] Implement `get_api_key` function
- [ ] Implement `create_api_key` function
- [ ] Implement `update_api_key` function
- [ ] Implement `delete_api_key` function

### 2. Create MCP Tools for API Keys Management

- [ ] Implement `List_API_Keys` tool
- [ ] Implement `Get_API_Key` tool
- [ ] Implement `Create_API_Key` tool
- [ ] Implement `Update_API_Key` tool
- [ ] Implement `Delete_API_Key` tool

### 3. Create Models for API Keys

- [ ] Define `ApiKey` model
- [ ] Define `ApiKeyRequest` model
- [ ] Define `ApiKeyResponse` model
- [ ] Define API key-related enums (e.g., `ApiKeyType`, `ExpirationPeriod`)

## Dependencies

- Authentication module (`auth.py`) for JWT handling
- Hyperswitch API Client
- Proper error handling with logging
- Validation utilities for API key data

## Technical Design

### API Keys Management Flow

1. **List API Keys**:
   - Authenticate with JWT token
   - Call GET `/api_keys`
   - Parse and return list of API keys (with sensitive parts masked)

2. **Get API Key Details**:
   - Authenticate with JWT token
   - Call GET `/api_keys/{key_id}`
   - Parse and return API key details (with sensitive parts masked)

3. **Create API Key**:
   - Authenticate with JWT token
   - Validate API key request data
   - Call POST `/api_keys`
   - Return created API key details including the newly generated key (only shown once)
   - Store the key securely for the user (displayed only once)

4. **Update API Key**:
   - Authenticate with JWT token
   - Validate API key update data
   - Call PUT `/api_keys/{key_id}`
   - Return updated API key details

5. **Delete API Key**:
   - Authenticate with JWT token
   - Call DELETE `/api_keys/{key_id}`
   - Return success/failure status

### Data Models

```python
from enum import Enum
from typing import Optional, Dict, Any, List
from datetime import datetime

class ApiKeyType(Enum):
    """Type of API key"""
    SECRET = "secret"
    PUBLISHABLE = "publishable"
    
class ExpirationPeriod(Enum):
    """API key expiration period"""
    NEVER = "never"
    THIRTY_DAYS = "30_days"
    NINETY_DAYS = "90_days"
    ONE_YEAR = "1_year"
    CUSTOM = "custom"

class ApiKey:
    """API key model"""
    key_id: str
    name: str
    description: Optional[str]
    prefix: str
    key_type: ApiKeyType
    expiration: ExpirationPeriod
    expiration_date: Optional[datetime]
    created_at: datetime
    merchant_id: str
    is_active: bool
    last_used: Optional[datetime]
    metadata: Optional[Dict[str, Any]]
    
class ApiKeyRequest:
    """API key creation/update request"""
    name: str
    description: Optional[str]
    key_type: ApiKeyType = ApiKeyType.SECRET
    expiration: ExpirationPeriod = ExpirationPeriod.NEVER
    expiration_date: Optional[datetime] = None
    metadata: Optional[Dict[str, Any]] = None
    
class ApiKeyResponse:
    """API key response with full key (only for creation)"""
    key_id: str
    name: str
    description: Optional[str]
    prefix: str
    full_key: Optional[str]  # Only populated on creation
    key_type: ApiKeyType
    expiration: ExpirationPeriod
    expiration_date: Optional[datetime]
    created_at: datetime
    merchant_id: str
    is_active: bool
    last_used: Optional[datetime]
    metadata: Optional[Dict[str, Any]]
```

## Security Considerations

- API keys are sensitive credentials and must be handled securely
- Full API keys should be displayed to users only once during creation
- API keys should be stored securely by users (not by the system in plaintext)
- When listing API keys, only show the prefix (e.g., "sk_test_123...")
- Implement rate limiting to prevent brute force attacks
- Log API key creation, updates, and deletion for audit purposes
- Mask API keys in logs and error messages
- Implement proper validation for API key requests

## Testing Approach

1. Test each API key endpoint individually
2. Verify proper JWT token handling
3. Test validation of API key data
4. Test error scenarios (invalid requests, duplicate names, etc.)
5. Test API key lifecycle (create, list, get, update, delete)
6. Verify only the prefix is shown when listing keys
7. Verify the full key is only shown once during creation

## Implementation Considerations

- API key creation should generate cryptographically secure random keys
- Keys should follow a format that identifies their type (e.g., "sk_test_..." for secret test keys)
- API key updates should not allow changing the key type
- Consider implementing API key rotation functionality
- Track API key usage for monitoring and security purposes

## Deliverables

1. Completed `api_keys.py` module with all functions
2. MCP tools for all API key management endpoints
3. Data models for API key entities
4. Unit tests for each function
5. Integration tests for complete API key management flows
6. Documentation for each endpoint and function 