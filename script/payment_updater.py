#!/usr/bin/env python3
"""
Payment Manual Update Script

This script reads payment data from extracted_request_ids.csv and makes
HTTP PUT requests to update payment records via the manual-update API.
"""

import csv
import json
import requests
import time
from pathlib import Path
from typing import Dict, Any

# Configuration
BASE_URL = "http://localhost:8080" 
API_KEY = "test_admin"
ATTEMPT_STATUS = "pending"

# Rate limiting (requests per second)
RATE_LIMIT = 10  # Adjust as needed
REQUEST_DELAY = 1.0 / RATE_LIMIT

def make_payment_update_request(payment_data: Dict[str, str]) -> Dict[str, Any]:
    """
    Make a PUT request to update payment status
    
    Args:
        payment_data: Dictionary containing payment fields
        
    Returns:
        Dictionary with request result
    """
    payment_id = payment_data['payment_id']
    merchant_id = payment_data['merchant_id'] if payment_data['merchant_id'] else 'default_merchant'
    attempt_id = payment_data['attempt_id']
    connector_transaction_id = payment_data['connector_transaction_id']
    
    # Construct URL
    url = f"{BASE_URL}/payments/{payment_id}/manual-update"
    
    # Headers
    headers = {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
        'X-Merchant-Id': merchant_id,
        'api-key': API_KEY
    }
    
    # Payload
    payload = {
        "attempt_id": attempt_id,
        "merchant_id": merchant_id,
        "attempt_status": ATTEMPT_STATUS,
        "connector_transaction_id": connector_transaction_id
    }
    
    try:
        response = requests.put(url, headers=headers, json=payload, timeout=30)
        
        return {
            'success': response.status_code in [200, 201, 202, 204],
            'status_code': response.status_code,
            'response_text': response.text[:200],  # Truncate for logging
            'payment_id': payment_id
        }
        
    except requests.exceptions.RequestException as e:
        return {
            'success': False,
            'status_code': None,
            'response_text': str(e),
            'payment_id': payment_id
        }

def process_payments(csv_file_path: str, output_log_path: str = None):
    """
    Process all payments from CSV file
    
    Args:
        csv_file_path: Path to the CSV file
        output_log_path: Optional path for detailed log output
    """
    if output_log_path is None:
        output_log_path = str(Path(csv_file_path).parent / "payment_update_log.txt")
    
    success_count = 0
    error_count = 0
    total_count = 0
    
    # Open log file for writing
    with open(output_log_path, 'w', encoding='utf-8') as log_file:
        log_file.write(f"Payment Update Log - Started at {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        log_file.write("=" * 60 + "\n\n")
        
        try:
            with open(csv_file_path, 'r', encoding='utf-8') as csvfile:
                reader = csv.DictReader(csvfile)
                
                print("Payment Manual Update Script")
                print("=" * 40)
                print(f"Reading from: {csv_file_path}")
                print(f"Logging to: {output_log_path}")
                print(f"Target API: {BASE_URL}")
                print(f"Rate limit: {RATE_LIMIT} req/sec")
                print()
                
                for row_num, row in enumerate(reader, 1):
                    total_count += 1
                    
                    # Skip rows with missing required data
                    if not all([row.get('payment_id'), row.get('connector_transaction_id'), row.get('attempt_id')]):
                        print(f"Row {row_num}: Skipping - missing required fields")
                        log_file.write(f"Row {row_num}: SKIPPED - Missing required fields: {row}\n")
                        error_count += 1
                        continue
                    
                    # Make the API request
                    result = make_payment_update_request(row)
                    
                    if result['success']:
                        success_count += 1
                        status_msg = f"Row {row_num}: SUCCESS - {result['payment_id']} (HTTP {result['status_code']})"
                        print(status_msg)
                        log_file.write(f"{status_msg}\n")
                    else:
                        error_count += 1
                        error_msg = f"Row {row_num}: ERROR - {result['payment_id']} (HTTP {result['status_code']}) - {result['response_text']}"
                        print(error_msg)
                        log_file.write(f"{error_msg}\n")
                    
                    # Progress updates every 100 requests
                    if total_count % 100 == 0:
                        print(f"Progress: {total_count} processed, {success_count} successful, {error_count} errors")
                    
                    # Rate limiting
                    time.sleep(REQUEST_DELAY)
                
        except FileNotFoundError:
            error_msg = f"Error: CSV file not found at {csv_file_path}"
            print(error_msg)
            log_file.write(f"{error_msg}\n")
            return
        except Exception as e:
            error_msg = f"Unexpected error: {e}"
            print(error_msg)
            log_file.write(f"{error_msg}\n")
            return
        
        # Final summary
        summary = f"""
Final Summary:
=============
Total processed: {total_count}
Successful: {success_count}
Errors: {error_count}
Success rate: {(success_count/total_count*100):.1f}% if total_count > 0 else 0.0%
Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}
"""
        
        print(summary)
        log_file.write(summary)
        
        if success_count > 0:
            print(f"🎉 Successfully updated {success_count} payments!")
        if error_count > 0:
            print(f"⚠️  {error_count} requests failed - check log for details")

def main():
    """
    Main function to run the payment updater
    """
    # Default paths
    script_dir = Path(__file__).parent
    csv_file = script_dir / "sample_request_ids.csv"
    
    print("Payment Manual Update Script")
    print("=" * 40)
    
    if not csv_file.exists():
        print(f"Error: CSV file not found at {csv_file}")
        print("Please ensure 'extracted_request_ids.csv' exists in the script directory")
        print("Run extract_request_ids.py first to generate the CSV file")
        return
    
    # Confirm before proceeding
    try:
        with open(csv_file, 'r') as f:
            reader = csv.DictReader(f)
            row_count = sum(1 for _ in reader)
        
        print(f"Found {row_count} rows in CSV file")
        print(f"This will make up to {row_count} API requests to {BASE_URL}")
        print(f"Estimated time: ~{(row_count * REQUEST_DELAY / 60):.1f} minutes")
        print()
        
        confirm = input("Continue? (y/N): ").lower().strip()
        if confirm != 'y':
            print("Operation cancelled")
            return
            
    except Exception as e:
        print(f"Error reading CSV file: {e}")
        return
    
    # Process the payments
    process_payments(str(csv_file))

if __name__ == "__main__":
    main()
