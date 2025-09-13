#!/usr/bin/env python3
"""
CSV Payment Data Parser

This script extracts specific fields from payment CSV data:
1. Direct fields: payment_id, created_at, flow
2. From Response JSON: status, id
3. From Request JSON: processingInformation.capture
"""

import csv
import json
import re
import os
from pathlib import Path

def clean_json_string(json_str):
    """
    Clean JSON string by replacing masked values with null
    """
    if not json_str or json_str.strip() == "":
        return "{}"
    
    # Replace the masked string patterns with null
    cleaned = re.sub(r'"?\*\*\*[^"]*\*\*\*"?', 'null', json_str)
    
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
    except (json.JSONDecodeError, TypeError, AttributeError) as e:
        print(f"Warning: Could not parse JSON: {e}")
        # For malformed JSON, try direct regex extraction instead
        return None

def extract_value_by_regex(json_str, field_path):
    """
    Extract specific values from JSON string using regex when JSON parsing fails
    """
    if field_path == "status":
        match = re.search(r'"status"\s*:\s*"([^"]*)"', json_str)
        return match.group(1) if match else None
    elif field_path == "id":
        match = re.search(r'"id"\s*:\s*"([^"]*)"', json_str)
        return match.group(1) if match else None
    elif field_path == "processingInformation.capture":
        match = re.search(r'"capture"\s*:\s*(true|false|null)', json_str)
        if match:
            value = match.group(1)
            return True if value == "true" else False if value == "false" else None
        return None
    return None

def extract_nested_value(data, path):
    """
    Extract nested value from dictionary using dot notation path
    Example: extract_nested_value(data, "processingInformation.capture")
    """
    keys = path.split('.')
    current = data
    
    for key in keys:
        if isinstance(current, dict) and key in current:
            current = current[key]
        else:
            return None
    
    return current

def parse_payment_csv(input_file, output_file):
    """
    Parse payment CSV and extract required fields
    """
    # Define output columns
    output_columns = [
        'payment_id',
        'created_at', 
        'flow',
        'response_status',
        'response_id',
        'request_capture'
    ]
    
    processed_count = 0
    error_count = 0
    
    try:
        with open(input_file, 'r', newline='', encoding='utf-8') as infile:
            # Try to detect delimiter (tab or comma)
            sample = infile.read(1024)
            infile.seek(0)
            delimiter = '\t' if '\t' in sample else ','
            
            reader = csv.DictReader(infile, delimiter=delimiter)
            
            with open(output_file, 'w', newline='', encoding='utf-8') as outfile:
                writer = csv.DictWriter(outfile, fieldnames=output_columns)
                writer.writeheader()
                
                for row_num, row in enumerate(reader, 1):
                    try:
                        # Extract basic fields
                        output_row = {
                            'payment_id': row.get('payment_id', ''),
                            'created_at': row.get('created_at', ''),
                            'flow': row.get('flow', ''),
                            'response_status': None,
                            'response_id': None,
                            'request_capture': None
                        }
                        
                        # Parse Response JSON for status and id
                        response_json_str = row.get('response', '')
                        if response_json_str:
                            response_data = safe_json_parse(response_json_str)
                            if response_data is not None:
                                # JSON parsing successful
                                output_row['response_status'] = response_data.get('status')
                                output_row['response_id'] = response_data.get('id')
                            else:
                                # JSON parsing failed, use regex
                                output_row['response_status'] = extract_value_by_regex(response_json_str, 'status')
                                output_row['response_id'] = extract_value_by_regex(response_json_str, 'id')
                        
                        # Parse Request JSON for processingInformation.capture
                        request_json_str = row.get('request', '')
                        if request_json_str:
                            request_data = safe_json_parse(request_json_str)
                            if request_data is not None:
                                # JSON parsing successful
                                output_row['request_capture'] = extract_nested_value(
                                    request_data, 'processingInformation.capture'
                                )
                            else:
                                # JSON parsing failed, use regex
                                output_row['request_capture'] = extract_value_by_regex(request_json_str, 'processingInformation.capture')
                        
                        writer.writerow(output_row)
                        processed_count += 1
                        
                    except Exception as e:
                        print(f"Error processing row {row_num}: {e}")
                        error_count += 1
                        continue
    
    except FileNotFoundError:
        print(f"Error: Input file '{input_file}' not found")
        return False
    except Exception as e:
        print(f"Error reading file: {e}")
        return False
    
    print(f"✅ Processing complete!")
    print(f"   Processed: {processed_count} rows")
    print(f"   Errors: {error_count} rows")
    print(f"   Output written to: {output_file}")
    
    return True

def main():
    """
    Main function to run the parser
    """
    # Input file from script directory
    script_dir = Path(__file__).parent
    input_file = script_dir / "ce.csv"
    
    # Output file in current script directory
    script_dir = Path(__file__).parent
    output_file = script_dir / "parsed_payments_output.csv"
    
    print("Payment CSV Parser")
    print("=" * 40)
    print(f"Input file: {input_file}")
    print(f"Output file: {output_file}")
    print()
    
    if not input_file.exists():
        print(f"❌ Error: Input file not found at {input_file}")
        print("Please ensure 'ce.csv' is in the script directory")
        return
    
    # Run the parser
    success = parse_payment_csv(str(input_file), str(output_file))
    
    if success:
        print(f"\n🎉 Successfully parsed payment data!")
        print(f"Check the output file: {output_file}")
    else:
        print(f"\n❌ Failed to parse payment data")

if __name__ == "__main__":
    main()
