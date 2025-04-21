#!/usr/bin/env python3
"""
Unit Tests for Authentication and User Management Modules

This script contains unit tests for the auth.py and user.py modules,
testing individual functions with both valid and invalid inputs.

Usage:
    python -m unittest test_auth_user_modules.py
"""

import unittest
import json
import os
from unittest.mock import patch, MagicMock

# Import the modules to test
from hyperswitch_mcp.auth import signin, terminate_2fa, signout
from hyperswitch_mcp.user import get_user_info, update_user
from hyperswitch_mcp.utils import initialize_logging, LogLevel


class MockResponse:
    """Mock class for requests.Response"""
    def __init__(self, status_code, json_data=None, content=None):
        self.status_code = status_code
        self._json_data = json_data
        self.content = content if content is not None else b''
        self.text = content.decode('utf-8') if content is not None else ''
        self.headers = {}
    
    def json(self):
        if self._json_data is None:
            raise ValueError("No JSON data")
        return self._json_data


class TestAuthModule(unittest.TestCase):
    """Test cases for auth.py module"""
    
    def setUp(self):
        """Set up test environment"""
        initialize_logging(LogLevel.INFO)
        # Sample valid responses
        self.valid_signin_response = {
            "token": "totp_sample_token",
            "user_id": "user_12345",
            "email": "test@example.com"
        }
        self.valid_terminate_response = {
            "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
            "user_id": "user_12345",
            "email": "test@example.com"
        }
        self.valid_signout_response = {
            "status": "success",
            "message": "Successfully signed out"
        }
    
    @patch('hyperswitch_mcp.auth.requests.post')
    def test_signin_valid(self, mock_post):
        """Test signin with valid credentials"""
        # Configure mock
        mock_post.return_value = MockResponse(200, self.valid_signin_response, 
                                              json.dumps(self.valid_signin_response).encode())
        
        # Call function
        result = signin("test@example.com", "password")
        
        # Assert
        self.assertIn("token", result)
        self.assertEqual(result["token"], "totp_sample_token")
        self.assertEqual(result["user_id"], "user_12345")
        self.assertEqual(result["email"], "test@example.com")
    
    @patch('hyperswitch_mcp.auth.requests.post')
    def test_signin_invalid(self, mock_post):
        """Test signin with invalid credentials"""
        # Configure mock
        error_response = {"error": "Invalid credentials"}
        mock_post.return_value = MockResponse(401, error_response, 
                                              json.dumps(error_response).encode())
        
        # Call function
        result = signin("test@example.com", "wrong_password")
        
        # Assert
        self.assertIn("error", result)
        self.assertIn("Sign-in failed", result["error"])
        self.assertEqual(result["status"], 401)
    
    @patch('hyperswitch_mcp.auth.requests.get')
    def test_terminate_2fa_valid(self, mock_get):
        """Test terminate_2fa with valid token"""
        # Configure mock
        mock_get.return_value = MockResponse(200, self.valid_terminate_response, 
                                              json.dumps(self.valid_terminate_response).encode())
        
        # Call function
        result = terminate_2fa("totp_sample_token", True)
        
        # Assert
        self.assertIn("user_info_token", result)
        self.assertEqual(result["user_info_token"], "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")
        self.assertEqual(result["user_id"], "user_12345")
    
    @patch('hyperswitch_mcp.auth.requests.get')
    def test_terminate_2fa_invalid(self, mock_get):
        """Test terminate_2fa with invalid token"""
        # Configure mock
        error_response = {"error": "Invalid token"}
        mock_get.return_value = MockResponse(401, error_response, 
                                              json.dumps(error_response).encode())
        
        # Call function
        result = terminate_2fa("invalid_totp_token")
        
        # Assert
        self.assertIn("error", result)
        self.assertIn("2FA termination failed", result["error"])
        self.assertEqual(result["status"], 401)
    
    @patch('hyperswitch_mcp.auth.requests.post')
    def test_signout_valid(self, mock_post):
        """Test signout with valid token"""
        # Configure mock
        mock_post.return_value = MockResponse(200, self.valid_signout_response, 
                                              json.dumps(self.valid_signout_response).encode())
        
        # Call function
        result = signout("jwt_token_sample")
        
        # Assert
        self.assertEqual(result["status"], "success")
        self.assertEqual(result["message"], "Successfully signed out")
    
    @patch('hyperswitch_mcp.auth.requests.post')
    def test_signout_invalid(self, mock_post):
        """Test signout with invalid token"""
        # Configure mock
        error_response = {"error": "Invalid token"}
        mock_post.return_value = MockResponse(401, error_response, 
                                              json.dumps(error_response).encode())
        
        # Call function
        result = signout("invalid_jwt_token")
        
        # Assert
        self.assertIn("error", result)
        self.assertIn("Sign-out failed", result["error"])
        self.assertEqual(result["status"], 401)


class TestUserModule(unittest.TestCase):
    """Test cases for user.py module"""
    
    def setUp(self):
        """Set up test environment"""
        initialize_logging(LogLevel.INFO)
        # Sample valid responses
        self.valid_user_info_response = {
            "user_id": "user_12345",
            "email": "test@example.com",
            "name": "Test User",
            "created_at": "2023-01-01T12:00:00Z",
            "roles": ["admin"],
            "status": "active"
        }
        self.valid_update_response = {
            "user_id": "user_12345",
            "email": "test@example.com",
            "name": "Updated Name",
            "phone": "+1234567890",
            "updated_at": "2023-01-01T12:00:00Z"
        }
    
    @patch('hyperswitch_mcp.user.requests.get')
    def test_get_user_info_valid(self, mock_get):
        """Test get_user_info with valid token"""
        # Configure mock
        mock_get.return_value = MockResponse(200, self.valid_user_info_response, 
                                              json.dumps(self.valid_user_info_response).encode())
        
        # Call function
        result = get_user_info("jwt_token_sample")
        
        # Assert
        self.assertEqual(result["user_id"], "user_12345")
        self.assertEqual(result["email"], "test@example.com")
        self.assertEqual(result["name"], "Test User")
        self.assertEqual(result["status"], "active")
    
    @patch('hyperswitch_mcp.user.requests.get')
    def test_get_user_info_invalid(self, mock_get):
        """Test get_user_info with invalid token"""
        # Configure mock
        error_response = {"error": "Invalid token"}
        mock_get.return_value = MockResponse(401, error_response, 
                                              json.dumps(error_response).encode())
        
        # Call function
        result = get_user_info("invalid_jwt_token")
        
        # Assert
        self.assertIn("error", result)
        self.assertIn("Failed to get user info", result["error"])
        self.assertEqual(result["status"], 401)
    
    # @patch('hyperswitch_mcp.user.requests.put')
    # def test_update_user_valid(self, mock_put):
    #     """Test update_user with valid data"""
    #     # Configure mock
    #     mock_put.return_value = MockResponse(200, self.valid_update_response, 
    #                                           json.dumps(self.valid_update_response).encode())
        
    #     # Call function
    #     result = update_user("jwt_token_sample", name="Updated Name", phone="+1234567890")
        
    #     # Assert
    #     self.assertEqual(result["user_id"], "user_12345")
    #     self.assertEqual(result["name"], "Updated Name")
    #     self.assertEqual(result["phone"], "+1234567890")
    
    @patch('hyperswitch_mcp.user.requests.put')
    def test_update_user_invalid_token(self, mock_put):
        """Test update_user with invalid token"""
        # Configure mock
        error_response = {"error": "Invalid token"}
        mock_put.return_value = MockResponse(401, error_response, 
                                              json.dumps(error_response).encode())
        
        # Call function
        result = update_user("invalid_jwt_token", name="Updated Name")
        
        # Assert
        self.assertIn("error", result)
        self.assertIn("Failed to update user", result["error"])
        self.assertEqual(result["status"], 401)
    
    def test_update_user_no_fields(self):
        """Test update_user with no fields to update"""
        # Call function
        result = update_user("jwt_token_sample")
        
        # Assert
        self.assertIn("error", result)
        self.assertEqual(result["error"], "No update fields provided")


if __name__ == '__main__':
    unittest.main() 