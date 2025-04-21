import pytest
from unittest.mock import patch, MagicMock

# Assuming the structure allows this import
# Adjust the path if necessary based on how tests are run (e.g., from the parent dir)
from hyperswitch_mcp.auth import signin, signout, terminate_2fa

# --- Test signin --- 

@patch('hyperswitch_mcp.auth.requests.post')
def test_signin_success(mock_post):
    """Test successful signin"""
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {"token_type": "totp", "totp_token": "fake_token", "user_id": "user1"}
    mock_post.return_value = mock_response

    result = signin("test@example.com", "password")

    assert "error" not in result
    assert result.get("totp_token") == "fake_token"
    mock_post.assert_called_once()
    # Further assertions could check the URL and payload passed to mock_post

@patch('hyperswitch_mcp.auth.requests.post')
def test_signin_failure_401(mock_post):
    """Test failed signin (401 Unauthorized)"""
    mock_response = MagicMock()
    mock_response.status_code = 401
    mock_response.json.return_value = {"error": {"message": "Invalid credentials"}}
    mock_post.return_value = mock_response

    result = signin("test@example.com", "wrong_password")

    assert "error" in result
    assert result.get("status") == 401
    assert "Invalid credentials" in result.get("details", {}).get("error", {}).get("message", "")
    mock_post.assert_called_once()

# --- Test terminate_2fa --- 

@patch('hyperswitch_mcp.auth.requests.get')
def test_terminate_2fa_success(mock_get):
    """Test successful 2FA termination"""
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {"token_type": "user_info", "token": "fake_jwt_token"}
    mock_get.return_value = mock_response

    result = terminate_2fa("fake_totp_token", skip_two_factor_auth=True)

    assert "error" not in result
    assert result.get("user_info_token") == "fake_jwt_token"
    mock_get.assert_called_once()
    assert "skip_two_factor_auth=true" in mock_get.call_args[0][0] # Check URL param

@patch('hyperswitch_mcp.auth.requests.get')
def test_terminate_2fa_failure_400(mock_get):
    """Test failed 2FA termination (e.g., invalid TOTP token)"""
    mock_response = MagicMock()
    mock_response.status_code = 400
    mock_response.json.return_value = {"error": {"message": "Invalid TOTP token"}}
    mock_get.return_value = mock_response

    result = terminate_2fa("invalid_totp_token")

    assert "error" in result
    assert result.get("status") == 400
    assert "Invalid TOTP token" in result.get("details", {}).get("error", {}).get("message", "")
    mock_get.assert_called_once()

# --- Test signout --- 

@patch('hyperswitch_mcp.auth.requests.post')
def test_signout_success(mock_post):
    """Test successful signout"""
    mock_response = MagicMock()
    mock_response.status_code = 200
    # Simulate empty JSON response body on success
    mock_response.json.side_effect = ValueError # Or return {} 
    mock_response.content = b'' # Empty content
    mock_post.return_value = mock_response

    result = signout("fake_jwt_token")

    assert "error" not in result
    assert result.get("status") == "success"
    assert result.get("message") == "Successfully signed out"
    mock_post.assert_called_once()

@patch('hyperswitch_mcp.auth.requests.post')
def test_signout_failure_401(mock_post):
    """Test failed signout (e.g., invalid JWT)"""
    mock_response = MagicMock()
    mock_response.status_code = 401
    mock_response.json.return_value = {"error": {"message": "Invalid token"}}
    mock_post.return_value = mock_response

    result = signout("invalid_jwt_token")

    assert "error" in result
    assert result.get("status") == 401
    assert "Invalid token" in result.get("details", {}).get("error", {}).get("message", "")
    mock_post.assert_called_once() 