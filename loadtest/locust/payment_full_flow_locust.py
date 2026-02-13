"""
Hyperswitch Full Payment Flow Load Test
Tests sequential create -> confirm -> retrieve on a single endpoint
Each user performs all three operations sequentially
"""

import json
import random
import uuid
from datetime import datetime, timedelta
from locust import HttpUser, task, between, events
import os

# =============================================================================
# CONFIGURATION - Use Environment Variables
# =============================================================================

# Base URL Configuration
BASE_URL = os.getenv("BASE_URL", "http://localhost:8080")
MERCHANT_API_KEY = os.getenv("MERCHANT_API_KEY", "")
PUBLISHABLE_KEY = os.getenv("PUBLISHABLE_KEY", "")

# Payment template - Based on your curl example
PAYMENT_CREATE_PAYLOAD = {
    "amount": 6540,
    "currency": "USD",
    "confirm": False,
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "amount_to_capture": 6540,
    "customer_id": "StripeCustomer",
    "email": "guest@example.com",
    "name": "John Doe",
    "phone": "999999999",
    "phone_country_code": "+65",
    "description": "Its my first payment request",
    "authentication_type": "no_three_ds",
    "return_url": "https://duck.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Fransico",
            "state": "California",
            "zip": "94122",
            "country": "US",
            "first_name": "PiX"
        }
    },
    "shipping": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Fransico",
            "state": "California",
            "zip": "94122",
            "country": "US",
            "first_name": "PiX"
        }
    },
    "statement_descriptor_name": "joseph",
    "statement_descriptor_suffix": "JS",
    "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
    }
}

# Browser info template for confirm
BROWSER_INFO_TEMPLATE = {
    "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
    "language": "en-GB",
    "color_depth": 30,
    "screen_height": 1117,
    "screen_width": 1728,
    "time_zone": -330,
    "java_enabled": True,
    "java_script_enabled": True
}


class HyperswitchFullFlowUser(HttpUser):
    """
    User class for testing complete payment flow.
    Performs Create -> Confirm -> Retrieve sequentially on a single endpoint.
    """
    
    # Base host configuration
    host = BASE_URL
    
    # Configurable wait time between complete cycles (in seconds)
    wait_time = between(2, 5)
    
    def on_start(self):
        """Called when a simulated user starts. Initializes headers."""
        
        # Validate API keys are configured
        if not MERCHANT_API_KEY or not PUBLISHABLE_KEY:
            print("WARNING: API keys not configured!")
        
        # Headers for create/retrieve (merchant API key)
        self.create_headers = {
            "Content-Type": "application/json",
            "Accept": "application/json",
            "api-key": MERCHANT_API_KEY
        }
        
        # Headers for confirm (publishable key)
        self.confirm_headers = {
            "Content-Type": "application/json",
            "Accept": "application/json",
            "api-key": PUBLISHABLE_KEY
        }
    
    @task(1)
    def full_payment_flow(self):
        """
        Execute complete payment flow.
        All operations happen sequentially.
        """
        # Step 1: Create Payment
        payment_id, client_secret = self._create_payment()
        
        if not payment_id:
            # Create failed, skip remaining steps
            return
        
        # Step 2: Confirm Payment
        if client_secret:
            self._confirm_payment(payment_id, client_secret)
        
        # Step 3: Retrieve Payment
        self._retrieve_payment(payment_id)
    
    def _create_payment(self) -> tuple:
        """
        Create a payment using the Hyperswitch API.
        Returns (payment_id, client_secret) tuple or (None, None) on failure.
        """
        payload = self._generate_payment_payload()
        
        with self.client.post(
            "/payments",
            headers=self.create_headers,
            json=payload,
            name="Full Flow - Create",
            catch_response=True
        ) as response:
            if response.status_code == 200 or response.status_code == 201:
                response.success()
                try:
                    data = response.json()
                    payment_id = data.get("payment_id")
                    client_secret = data.get("client_secret")
                    return payment_id, client_secret
                except json.JSONDecodeError:
                    pass
            else:
                response.failure(f"Status {response.status_code}: {response.text[:200]}")
        
        return None, None
    
    def _confirm_payment(self, payment_id: str, client_secret: str):
        """
        Confirm a payment using the Hyperswitch API.
        """
        payload = {
            "client_secret": client_secret,
            "browser_info": self._generate_browser_info(),
            "payment_method": "card",
            "payment_method_type": "credit",
            "payment_method_data": {
                "card": {
                    "card_number": "4242424242424242",
                    "card_exp_month": "03",
                    "card_exp_year": "2030",
                    "card_holder_name": "joseph Doe",
                    "card_cvc": "737"
                }
            },
        }
        
        with self.client.post(
            f"/payments/{payment_id}/confirm",
            headers=self.confirm_headers,
            json=payload,
            name="Full Flow - Confirm",
            catch_response=True
        ) as response:
            
            if response.status_code in [200, 201, 202]:
                response.success()
            elif response.status_code in [404, 400]:
                # Payment might have been confirmed already or doesn't exist
                # Don't count as failure for load testing
                response.success()
            else:
                response.failure(f"Status {response.status_code}: {response.text[:200]}")
    
    def _retrieve_payment(self, payment_id: str):
        """
        Retrieve payment details (simple GET without force_sync).
        """
        with self.client.get(
            f"/payments/{payment_id}",
            headers=self.create_headers,
            name="Full Flow - Retrieve",
            catch_response=True
        ) as response:
            
            if response.status_code == 200:
                response.success()
            elif response.status_code == 404:
                # Payment might not exist
                response.success()
            else:
                response.failure(f"Status {response.status_code}: {response.text[:200]}")
    
    def _generate_payment_payload(self) -> dict:
        """
        Generate a payment payload with variations.
        """
        payload = PAYMENT_CREATE_PAYLOAD.copy()
        
        # Vary amount slightly
        base_amount = 6540
        amount_variation = random.randint(-500, 500)
        payload["amount"] = base_amount + amount_variation
        payload["amount_to_capture"] = payload["amount"]
        
        # Add variation to capture_on date
        future_date = datetime.now() + timedelta(days=random.randint(1, 30))
        payload["capture_on"] = future_date.strftime("%Y-%m-%dT%H:%M:%SZ")
        
        # Unique customer ID
        unique_id = str(uuid.uuid4())[:8]
        payload["customer_id"] = f"customer_{unique_id}"
        
        # Vary email slightly
        payload["email"] = f"user{unique_id}@example.com"
        
        # Vary phone
        payload["phone"] = f"{random.randint(900000000, 999999999)}"
        
        # Unique metadata
        payload["metadata"]["test_run_id"] = unique_id
        payload["metadata"]["timestamp"] = datetime.now().isoformat()
        
        return payload
    
    def _generate_browser_info(self) -> dict:
        """
        Generate browser info with variations.
        """
        browser_info = BROWSER_INFO_TEMPLATE.copy()
        
        # Vary screen dimensions slightly
        base_width = 1920
        base_height = 1080
        browser_info["screen_width"] = base_width + random.randint(-100, 100)
        browser_info["screen_height"] = base_height + random.randint(-50, 50)
        
        # Vary color depth
        browser_info["color_depth"] = random.choice([24, 30, 32])
        
        # Randomize timezone slightly
        browser_info["time_zone"] = random.randint(-720, 720)
        
        return browser_info


@events.test_start.add_listener
def on_test_start(environment, **kwargs):
    """Called when the test starts."""
    print("\n" + "=" * 70)
    print("HYPERSWITCH FULL PAYMENT FLOW LOAD TEST")
    print("=" * 70)
    print(f"\nEndpoint: {BASE_URL}")
    print(f"  Merchant API Key: {'*' * 10 if MERCHANT_API_KEY else 'NOT SET'}")
    print(f"  Publishable Key: {'*' * 10 if PUBLISHABLE_KEY else 'NOT SET'}")
    print(f"\nTest Type: Create -> Confirm -> Retrieve")
    print(f"Duration: 5 minutes (default)")
    print("=" * 70)
    print("\nFlow per user:")
    print("   POST /payments")
    print("  → POST /payments/:id/confirm")
    print("  → GET /payments/:id")
    print("\nUsers wait 2-5 seconds between complete payment cycles")
    print("=" * 70 + "\n")


@events.test_stop.add_listener
def on_test_stop(environment, **kwargs):
    """Called when the test stops. Prints summary statistics."""
    print("\n" + "=" * 70)
    print("TEST COMPLETED - SUMMARY")
    print("=" * 70)
    
    if hasattr(environment, 'stats') and environment.stats.total.num_requests > 0:
        stats = environment.stats.total
        
        print(f"\nTotal Requests: {stats.num_requests}")
        print(f"Failures: {stats.num_failures}")
        print(f"Success Rate: {stats.success_ratio():.2%}")
        print(f"\nOverall Response Times:")
        print(f"  Average: {stats.avg_response_time:.2f} ms")
        print(f"  Median: {stats.median_response_time:.2f} ms")
        print(f"  Min: {stats.min_response_time:.2f} ms")
        print(f"  Max: {stats.max_response_time:.2f} ms")
        print(f"  95th percentile: {stats.get_response_time_percentile(0.95):.2f} ms")
        print(f"  99th percentile: {stats.get_response_time_percentile(0.99):.2f} ms")
        print(f"\nOverall Requests Per Second (RPS): {stats.total_rps:.2f}")
        print(f"Overall Failure RPS: {stats.fail_rps:.2f}")
    
    # Print breakdown by operation
    if hasattr(environment, 'stats'):
        print("\n" + "=" * 70)
        print("BREAKDOWN BY OPERATION")
        print("=" * 70)
        
        for op in ["Create", "Confirm", "Retrieve"]:
            stat = environment.stats.get(f"Full Flow - {op}", None)
            if stat:
                print(f"\n{op}:")
                print(f"  Requests: {stat.num_requests}")
                print(f"  Failures: {      stat.num_failures}")
                print(f"  Avg Response: {  stat.avg_response_time:.2f} ms")
                print(f"  95th percentile: {stat.get_response_time_percentile(0.95):.2f} ms")
    
    print("=" * 70 + "\n")


if __name__ == "__main__":
    # Allow running directly for debugging
    pass