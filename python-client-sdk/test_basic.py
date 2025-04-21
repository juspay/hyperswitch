import requests
import time

def test_basic_flow():
    # 1. Test health endpoint
    print("\nChecking health endpoint...")
    health_response = requests.get("http://localhost:8080/health")
    print(f"Health Status: {health_response.status_code}")
    
    # 2. Create merchant account
    print("\nCreating merchant account...")
    try:
        # Generate unique merchant ID using timestamp
        merchant_id = f"test_merchant_{int(time.time())}"
        
        merchant_request = {
            "merchant_id": merchant_id,
            "merchant_name": "Test Merchant",
            "merchant_details": {
                "primary_contact_person": "John Doe",
                "primary_email": "john@example.com",
                "primary_phone": "1234567890",
                "website": "https://www.example.com",
                "about_business": "Test business",
                "address": {
                    "line1": "1234 Main St",
                    "city": "San Francisco",
                    "state": "CA",
                    "zip": "94122",
                    "country": "US"
                }
            }
        }

        response = requests.post(
            "http://localhost:8080/accounts",
            json=merchant_request,
            headers={
                "Content-Type": "application/json",
                "api-key": "test_admin"
            }
        )
        
        print(f"\nMerchant Creation Response Status: {response.status_code}")
        
        if response.status_code == 200:
            merchant_data = response.json()
            print("\nMerchant Details:")
            print(f"Merchant ID: {merchant_data.get('merchant_id')}")
            print(f"Publishable Key: {merchant_data.get('publishable_key')}")
            print(f"Organization ID: {merchant_data.get('organization_id')}")
        else:
            print("Error Response:", response.json())

    except Exception as e:
        print(f"Error occurred: {str(e)}")

if __name__ == "__main__":
    test_basic_flow() 