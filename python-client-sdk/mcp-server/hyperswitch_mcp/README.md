# Hyperswitch MCP Logging Utilities

This module provides a robust logging and debugging framework for the Hyperswitch MCP server. The utilities are designed to provide consistent, structured logging throughout the application, with support for different log levels, context information, and API request/response logging.

## Features

- **Configurable Log Levels**: DEBUG, INFO, WARNING, ERROR, NONE
- **Context-Rich Logging**: Includes file, function, line number, and timestamp
- **Structured JSON Context**: Attach structured data to log messages
- **API Request/Response Debugging**: Special utilities for logging API interactions
- **Performance Monitoring**: Timing decorators for measuring function execution
- **Sensitive Data Masking**: Automatically masks passwords, tokens, and other sensitive data

## Usage

### Basic Logging

```python
from hyperswitch_mcp.utils import Logger, LogLevel

# Initialize logging (typically done in main app entry point)
from hyperswitch_mcp.utils import initialize_logging
initialize_logging(LogLevel.DEBUG)

# Log at different levels
Logger.debug("This is a debug message")
Logger.info("This is an info message")
Logger.warning("This is a warning message")
Logger.error("This is an error message")

# Log with context data
Logger.info("User signed in", {"user_id": "123", "ip_address": "192.168.1.1"})

# Log exceptions
try:
    1 / 0
except Exception as e:
    Logger.exception("Division error occurred", e)
```

### API Debugging

```python
from hyperswitch_mcp.utils import ApiDebugger

# Log an API request
ApiDebugger.log_request(
    "POST", 
    "https://api.example.com/endpoint", 
    {"Content-Type": "application/json", "Authorization": "Bearer token123"},
    {"param1": "value1", "password": "secret"}  # Password will be masked in logs
)

# Log an API response
ApiDebugger.log_response(
    200,  # Status code
    {"Content-Type": "application/json"},  # Headers
    b'{"result": "success"}',  # Response body (bytes)
    0.325  # Elapsed time in seconds
)
```

### Performance Timing

```python
from hyperswitch_mcp.utils import timed_execution

# As a decorator
@timed_execution("Database Query")
def fetch_data(query):
    # Function execution time will be logged
    return database.execute(query)

# Or with a default name (uses function name)
@timed_execution()
def process_payment():
    # Logs will show "Starting process_payment" and 
    # "Completed process_payment in X.XXX seconds"
    pass
```

## Configuration

To change log level at runtime:

```python
from hyperswitch_mcp.utils import Logger, LogLevel

# Set to WARNING level (will only log WARNING and ERROR)
Logger.set_log_level(LogLevel.WARNING)

# Disable logging entirely
Logger.set_log_level(LogLevel.NONE)
```

To change the log output destination:

```python
import sys
from hyperswitch_mcp.utils import Logger

# Log to a file instead of stderr
log_file = open('app.log', 'a')
Logger.set_log_file(log_file)
``` 