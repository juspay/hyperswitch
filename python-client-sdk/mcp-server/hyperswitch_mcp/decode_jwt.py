import base64
import json

# The user_info_token obtained previously
user_info_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VyX2lkIjoiMzRkNmY1MjEtMWJmNS00MzYzLTg3MzAtMzk1ZjA1YzY3YzlkIiwibWVyY2hhbnRfaWQiOiJtZXJjaGFudF8xNzQ1MDE1NzM0Iiwicm9sZV9pZCI6Im9yZ19hZG1pbiIsImV4cCI6MTc0NTI3NzMzNCwib3JnX2lkIjoib3JnX0NsTVVmelN1M01Udm5scWFkWU9SIiwicHJvZmlsZV9pZCI6InByb19USHBmTUlaRm9RbnhteGFzQmExMCIsInRlbmFudF9pZCI6InB1YmxpYyJ9.CRswvt-B7N-ANJh-SqlwQFXppPPmUUpsMSMncVx9Eh4"

# Split the JWT into its parts (header, payload, signature)
try:
    header, payload_b64, signature = user_info_token.split('.')

    # The payload is the middle part. Base64 needs padding.
    # Add padding '=' if necessary (length must be multiple of 4)
    payload_b64 += '=' * (-len(payload_b64) % 4)

    # Decode the Base64 string (URL-safe variant)
    payload_bytes = base64.urlsafe_b64decode(payload_b64)

    # Decode the bytes into a UTF-8 string
    payload_json_str = payload_bytes.decode('utf-8')

    # Parse the JSON string into a Python dictionary
    payload_data = json.loads(payload_json_str)

    # Extract the merchant_id
    merchant_id = payload_data.get('merchant_id')

    if merchant_id:
        print(f"Successfully decoded payload.")
        print(f"Extracted merchant_id: {merchant_id}")
    else:
        print("Could not find 'merchant_id' in the decoded payload.")
        print("Decoded payload:", payload_data)

except ValueError:
    print("Error: Invalid JWT format. Could not split into 3 parts.")
except (base64.binascii.Error, TypeError) as e:
    print(f"Error decoding Base64: {e}")
except (UnicodeDecodeError, json.JSONDecodeError) as e:
    print(f"Error decoding JSON payload: {e}")
except Exception as e:
    print(f"An unexpected error occurred: {e}")
