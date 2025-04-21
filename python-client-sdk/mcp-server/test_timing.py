from hyperswitch_mcp.utils import timed_execution, Logger, LogLevel, initialize_logging
import time

# Initialize logging
initialize_logging(LogLevel.DEBUG)

# Rename function to avoid pytest collection
@timed_execution("Test Function")
def _example_timed_function():
    print("Function is running...")
    time.sleep(1)  # Simulate work
    return "Done!"

# Execute the function
# Update function call
result = _example_timed_function()
print(f"Result: {result}") 