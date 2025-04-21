from hyperswitch_mcp.utils import Logger, LogLevel, initialize_logging
import time

# Initialize logging with DEBUG level
initialize_logging(LogLevel.DEBUG)

def cause_exception():
    # Try to divide by zero to cause an exception
    try:
        print("About to cause an exception...")
        result = 1 / 0
        return result
    except Exception as e:
        # Log the exception
        Logger.exception("Error occurred during division", e, {
            "operation": "division",
            "numerator": 1,
            "denominator": 0
        })
        print("Exception caught and logged")

# Call the function
cause_exception() 