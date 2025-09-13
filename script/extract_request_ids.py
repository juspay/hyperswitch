#!/usr/bin/env python3
"""
Payment Data Extractor

This script extracts specific fields from payment CSV data with filtering:
- Filters: flow = "Authorize" OR "SetupMandate" AND response is non-empty
- Extracts: payment_id, id from response JSON, and clientReferenceInformation.code from request JSON
"""

import csv
import json
import re
from pathlib import Path

def clean_json_string(json_str):
    """
    Clean JSON string by replacing masked values with null
    """
    if not json_str or json_str.strip() == "":
        return "{}"
    
    # First handle CSV-style double quote escaping (""" -> ")
    cleaned = json_str.replace('""', '"')
    
    # Replace the masked string patterns with null
    cleaned = re.sub(r'"?\*\*\*[^"]*\*\*\*"?', 'null', cleaned)
    
    # Handle the specific issue with unescaped quotes in values
    # Pattern: "value":"brand=\"septapark\"" should become "value":"brand=\\\"septapark\\\""
    # This is a more targeted fix for the embedded quotes issue
    cleaned = re.sub(r':"([^"]*)"([^",}]*)"([^",}]*)"', r':"\1\\"\2\\"\3"', cleaned)
    
    # Also handle the case where there might be more complex embedded quotes
    # Look for patterns like "value":"key=\"value\"" and escape the inner quotes
    cleaned = re.sub(r':"([^"]*=\\*"[^"]*\\*"[^"]*)"', lambda m: f'":"{m.group(1).replace(chr(34), chr(92)+chr(34))}"', cleaned)
    
    # Handle additional cleaning for malformed JSON
    cleaned = cleaned.strip()
    
    # Fix null values that might not be properly quoted
    cleaned = re.sub(r':\s*null(?=\s*[,}])', ': null', cleaned)
    
    return cleaned

def safe_json_parse(json_str):
    """
    Safely parse JSON string with error handling
    """
    try:
        cleaned = clean_json_string(json_str)
        return json.loads(cleaned)
    except (json.JSONDecodeError, TypeError, AttributeError):
        # For malformed JSON, return None to trigger regex extraction
        return None

def extract_id_by_regex(json_str):
    """
    Extract ID from JSON string using regex when JSON parsing fails
    """
    # Look for "id" field at the top level of response JSON
    match = re.search(r'"id"\s*:\s*"([^"]*)"', json_str)
    return match.group(1) if match else None

def extract_client_reference_by_regex(json_str):
    """
    Extract clientReferenceInformation.code from JSON string using regex when JSON parsing fails
    """
    # Look for "code" field within clientReferenceInformation
    match = re.search(r'"code"\s*:\s*"([^"]*)"', json_str)
    return match.group(1) if match else None

def extract_request_ids(input_file, output_file):
    """
    Extract payment_id, request id, and client reference code from CSV with filtering and deduplication
    """
    # Define output columns
    output_columns = ['payment_id', 'connector_transaction_id', 'attempt_id', 'merchant_id']
    
    total_rows = 0
    filtered_rows = 0
    processed_rows = 0
    duplicate_count = 0
    error_count = 0
    
    # Set to track unique combinations we've already seen
    seen_combinations = set()
    
    try:
        with open(input_file, 'r', newline='', encoding='utf-8') as infile:
            # This CSV is comma-separated with quoted fields
            delimiter = ','
            
            reader = csv.DictReader(infile, delimiter=delimiter)
            
            with open(output_file, 'w', newline='', encoding='utf-8') as outfile:
                writer = csv.DictWriter(outfile, fieldnames=output_columns)
                writer.writeheader()
                
                for row_num, row in enumerate(reader, 1):
                    total_rows += 1
                    
                    try:
                        # Get fields for filtering and extraction
                        flow = row.get('flow', '').strip()
                        response_json_str = row.get('response', '').strip()
                        request_json_str = row.get('request', '').strip()
                        payment_id = row.get('payment_id', '').strip()
                        
                        # Apply filters
                        # 1. Check if flow is Authorize or SetupMandate
                        if flow not in ['Authorize', 'SetupMandate']:
                            continue
                            
                        # 2. Check if response is non-empty
                        if not response_json_str:
                            continue
                        
                        filtered_rows += 1
                        
                        # Extract response ID
                        connector_transaction_id = None
                        
                        if response_json_str:
                            # Try JSON parsing first
                            response_data = safe_json_parse(response_json_str)
                            if response_data is not None:
                                # JSON parsing successful - look for top-level "id"
                                connector_transaction_id = response_data.get('id')
                            else:
                                # JSON parsing failed, use regex
                                connector_transaction_id = extract_id_by_regex(response_json_str)
                        
                        # Extract client reference code from request
                        attempt_id = None
                        
                        if request_json_str:
                            # Try JSON parsing first
                            request_data = safe_json_parse(request_json_str)
                            if request_data is not None:
                                # JSON parsing successful - look for clientReferenceInformation.code
                                client_ref_info = request_data.get('clientReferenceInformation', {})
                                if isinstance(client_ref_info, dict):
                                    attempt_id = client_ref_info.get('code')
                            else:
                                # JSON parsing failed, use regex
                                attempt_id = extract_client_reference_by_regex(request_json_str)
                        
                        # Create combination key for deduplication (using all 3 fields)
                        combination_key = (payment_id, connector_transaction_id, attempt_id)
                        
                        # Check if this combination has already been seen
                        if combination_key in seen_combinations:
                            duplicate_count += 1
                            continue  # Skip this duplicate
                        
                        # Add to seen combinations
                        seen_combinations.add(combination_key)
                        
                        # Write the result
                        output_row = {
                            'payment_id': payment_id,
                            'connector_transaction_id': connector_transaction_id,
                            'attempt_id': attempt_id,
                            'merchant_id': '',
                        }
                        
                        writer.writerow(output_row)
                        processed_rows += 1
                        
                        # Progress indicator for large files
                        if processed_rows % 1000 == 0:
                            print(f"Processed {processed_rows} unique rows...")
                        
                    except Exception as e:
                        print(f"Error processing row {row_num}: {e}")
                        error_count += 1
                        continue
    
    except FileNotFoundError:
        print(f"❌ Error: Input file '{input_file}' not found")
        return False
    except Exception as e:
        print(f"❌ Error reading file: {e}")
        return False
    
    # Print summary
    print(f"\n✅ Processing complete!")
    print(f"   Total rows: {total_rows}")
    print(f"   Rows matching filters: {filtered_rows}")
    print(f"   Duplicates skipped: {duplicate_count}")
    print(f"   Unique rows processed: {processed_rows}")
    print(f"   Errors: {error_count}")
    print(f"   Output written to: {output_file}")
    
    return True

def main():
    """
    Main function to run the extractor
    """
    # Input file from script directory
    script_dir = Path(__file__).parent
    input_file = script_dir / "ce.csv"
    
    # Output file in current script directory
    output_file = script_dir / "extracted_request_ids.csv"
    
    print("Payment Data Extractor")
    print("=" * 40)
    print(f"Input file: {input_file}")
    print(f"Output file: {output_file}")
    print("Filters:")
    print("  - flow = 'Authorize' OR 'SetupMandate'")
    print("  - response is non-empty")
    print("Extracting: payment_id, connector_transaction_id, attempt_id, merchant_id")
    print()
    
    if not input_file.exists():
        print(f"❌ Error: Input file not found at {input_file}")
        print("Please ensure 'ce.csv' is in the script directory")
        return
    
    # Run the extractor
    success = extract_request_ids(str(input_file), str(output_file))
    
    if success:
        print(f"\n🎉 Successfully extracted request IDs!")
        print(f"Check the output file: {output_file}")
    else:
        print(f"\n❌ Failed to extract request IDs")

if __name__ == "__main__":
    main()
