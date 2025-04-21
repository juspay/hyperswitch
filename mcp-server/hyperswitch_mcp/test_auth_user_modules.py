from unittest.mock import patch, MagicMock
import json

@patch('hyperswitch_mcp.user.requests.post')
def test_update_user_valid(self, mock_post):
    """Test update_user with valid data"""
    # Configure mock
    mock_response_data = {"user_id": "user_12345", "name": "Updated Name", "phone": "+1234567890"} # Example success data
    # Assuming MockResponse is a custom helper or use MagicMock directly
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = mock_response_data
    mock_post.return_value = mock_response

    # Call function
    result = update_user("jwt_token_sample", name="Updated Name", phone="+1234567890")

    # Assert mock was called
    mock_post.assert_called_once()
    
    # Assert result based on successful mock
    self.assertNotIn("error", result)
    self.assertEqual(result["user_id"], "user_12345")

    # Assert
    self.assertEqual(result["user_id"], "user_12345") # This might still fail if API returns 401 