import pytest
from unittest.mock import patch, MagicMock

# Assuming the structure allows this import
# Adjust the path if necessary based on how tests are run
from hyperswitch_mcp.user import (
    get_user_info, update_user, change_password, 
    initiate_password_reset, reset_password_confirm, verify_email
)

# --- Test get_user_info --- 

# @patch('hyperswitch_mcp.user.requests.get')
# def test_get_user_info_success(mock_get):
#     """Test successful get_user_info"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"user_id": "user1", "email": "test@example.com", "name": "Test"}
#     mock_get.return_value = mock_response

#     result = get_user_info("fake_jwt")

#     assert "error" not in result
#     assert result.get("user_id") == "user1"
#     mock_get.assert_called_once()
#     # Assert correct URL called
#     assert mock_get.call_args[0][0] == "http://localhost:8080/user"

# @patch('hyperswitch_mcp.user.requests.get')
# def test_get_user_info_failure_401(mock_get):
#     """Test failed get_user_info (401 Unauthorized)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 401
#     mock_response.json.return_value = {"error": {"message": "Invalid JWT"}}
#     mock_get.return_value = mock_response

#     result = get_user_info("invalid_jwt")

#     assert "error" in result
#     assert result.get("status") == 401
#     assert "Invalid JWT" in result.get("details", {}).get("error", {}).get("message", "")
#     mock_get.assert_called_once()

# # --- Test update_user --- 

# @patch('hyperswitch_mcp.user.requests.put') # Assuming PUT, adjust if POST
# def test_update_user_success(mock_put):
#     """Test successful update_user"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"user_id": "user1", "name": "Updated Name"}
#     mock_put.return_value = mock_response

#     result = update_user("fake_jwt", name="Updated Name")

#     assert "error" not in result
#     assert result.get("name") == "Updated Name"
#     mock_put.assert_called_once()
#     # Assert payload contains name
#     assert mock_put.call_args[1]['json'] == {"name": "Updated Name"}

# @patch('hyperswitch_mcp.user.requests.put') # Assuming PUT
# def test_update_user_failure_400(mock_put):
#     """Test failed update_user (400 Bad Request)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 400
#     mock_response.json.return_value = {"error": {"message": "Invalid field"}}
#     mock_put.return_value = mock_response

#     result = update_user("fake_jwt", name="", phone="invalid-phone")

#     assert "error" in result
#     assert result.get("status") == 400
#     mock_put.assert_called_once()

# # --- Test change_password --- 

# @patch('hyperswitch_mcp.user.requests.post')
# def test_change_password_success(mock_post):
#     """Test successful change_password"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"message": "Password changed successfully"}
#     mock_post.return_value = mock_response

#     result = change_password("fake_jwt", "old_pass", "new_pass")

#     assert "error" not in result
#     assert result.get("status") == "success"
#     mock_post.assert_called_once()
#     # Assert URL and payload
#     assert mock_post.call_args[0][0] == "http://localhost:8080/user/change_password"
#     assert mock_post.call_args[1]['json'] == {"current_password": "old_pass", "new_password": "new_pass"}

# @patch('hyperswitch_mcp.user.requests.post')
# def test_change_password_failure_401(mock_post):
#     """Test failed change_password (Incorrect current password)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 401
#     mock_response.json.return_value = {"error": {"message": "Incorrect current password"}}
#     mock_post.return_value = mock_response

#     result = change_password("fake_jwt", "wrong_old_pass", "new_pass")

#     assert "error" in result
#     assert result.get("status") == 401
#     assert "Incorrect current password" in result.get("details", {}).get("error", {}).get("message", "")
#     mock_post.assert_called_once()

# # --- Test initiate_password_reset --- 

# @patch('hyperswitch_mcp.user.requests.post')
# def test_initiate_password_reset_success(mock_post):
#     """Test successful initiate_password_reset"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"message": "Reset email sent"}
#     mock_post.return_value = mock_response

#     result = initiate_password_reset("user@example.com")

#     assert "error" not in result
#     assert result.get("status") == "success"
#     mock_post.assert_called_once()
#     assert mock_post.call_args[1]['json'] == {"email": "user@example.com"}

# @patch('hyperswitch_mcp.user.requests.post')
# def test_initiate_password_reset_failure_404(mock_post):
#     """Test failed initiate_password_reset (User not found)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 404
#     mock_response.json.return_value = {"error": {"message": "User not found"}}
#     mock_post.return_value = mock_response

#     result = initiate_password_reset("notfound@example.com")

#     assert "error" in result
#     assert result.get("status") == 404
#     mock_post.assert_called_once()

# # --- Test reset_password_confirm --- 

# @patch('hyperswitch_mcp.user.requests.post')
# def test_reset_password_confirm_success(mock_post):
#     """Test successful reset_password_confirm"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"message": "Password reset successfully"}
#     mock_post.return_value = mock_response

#     result = reset_password_confirm("reset_token_123", "new_strong_pass")

#     assert "error" not in result
#     assert result.get("status") == "success"
#     mock_post.assert_called_once()
#     assert mock_post.call_args[1]['json'] == {"token": "reset_token_123", "new_password": "new_strong_pass"}

# @patch('hyperswitch_mcp.user.requests.post')
# def test_reset_password_confirm_failure_400(mock_post):
#     """Test failed reset_password_confirm (Invalid token)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 400
#     mock_response.json.return_value = {"error": {"message": "Invalid or expired token"}}
#     mock_post.return_value = mock_response

#     result = reset_password_confirm("invalid_token", "new_pass")

#     assert "error" in result
#     assert result.get("status") == 400
#     mock_post.assert_called_once()

# # --- Test verify_email --- 

# @patch('hyperswitch_mcp.user.requests.post')
# def test_verify_email_success(mock_post):
#     """Test successful verify_email"""
#     mock_response = MagicMock()
#     mock_response.status_code = 200
#     mock_response.json.return_value = {"message": "Email verified successfully"}
#     mock_post.return_value = mock_response

#     result = verify_email("verify_token_abc")

#     assert "error" not in result
#     assert result.get("status") == "success"
#     mock_post.assert_called_once()
#     assert mock_post.call_args[1]['json'] == {"token": "verify_token_abc"}

# @patch('hyperswitch_mcp.user.requests.post')
# def test_verify_email_failure_400(mock_post):
#     """Test failed verify_email (Invalid token)"""
#     mock_response = MagicMock()
#     mock_response.status_code = 400
#     mock_response.json.return_value = {"error": {"message": "Invalid or expired verification token"}}
#     mock_post.return_value = mock_response

#     result = verify_email("invalid_verify_token")

#     assert "error" in result
#     assert result.get("status") == 400
#     mock_post.assert_called_once() 