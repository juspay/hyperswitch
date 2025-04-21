import sys
import json
import traceback
import inspect
import time
from enum import Enum
from typing import Any, Dict, Optional, Union, List


class LogLevel(Enum):
    """Enum representing log levels"""
    DEBUG = 0
    INFO = 1
    WARNING = 2
    ERROR = 3
    NONE = 4  # For disabling logging


class Logger:
    """
    Logger utility for consistent logging across the application.
    
    Provides methods for logging at different levels with consistent formatting
    and context information.
    """
    
    # Default log level - can be changed at runtime
    CURRENT_LOG_LEVEL = LogLevel.INFO
    
    # Output file handle - defaults to stderr
    LOG_FILE = sys.stderr
    
    @classmethod
    def set_log_level(cls, level: LogLevel) -> None:
        """Set the current log level"""
        cls.CURRENT_LOG_LEVEL = level
        cls.debug(f"Log level set to {level.name}")
    
    @classmethod
    def set_log_file(cls, file_handle) -> None:
        """Set the file handle for logging output"""
        cls.LOG_FILE = file_handle
    
    @classmethod
    def _should_log(cls, level: LogLevel) -> bool:
        """Check if the given log level should be logged based on current settings"""
        return level.value >= cls.CURRENT_LOG_LEVEL.value
    
    @classmethod
    def _get_caller_info(cls) -> Dict[str, str]:
        """Get information about the caller of the logging function"""
        # Get the call stack
        stack = inspect.stack()
        # The caller of the logging method will be 2 frames up
        # (0 is this function, 1 is the logging method, 2 is the caller)
        if len(stack) > 2:
            frame = stack[2]
            return {
                "file": frame.filename.split("/")[-1],  # Just the filename, not the full path
                "function": frame.function,
                "line": frame.lineno
            }
        return {"file": "unknown", "function": "unknown", "line": 0}
    
    @classmethod
    def _format_log(cls, level: LogLevel, message: str, context: Optional[Dict[str, Any]] = None) -> str:
        """Format a log message with timestamp, level, and context information"""
        caller = cls._get_caller_info()
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime())
        
        # Base log parts
        log_parts = [
            f"[{timestamp}]",
            f"[{level.name}]",
            f"[{caller['file']}:{caller['function']}:{caller['line']}]",
            message
        ]
        
        # Add context as JSON if provided
        if context:
            try:
                context_str = json.dumps(context)
                log_parts.append(f"Context: {context_str}")
            except (TypeError, ValueError):
                log_parts.append(f"Context: (Unable to serialize context)")
        
        return " ".join(log_parts)
    
    @classmethod
    def debug(cls, message: str, context: Optional[Dict[str, Any]] = None) -> None:
        """Log a debug message"""
        if cls._should_log(LogLevel.DEBUG):
            log_str = cls._format_log(LogLevel.DEBUG, message, context)
            print(log_str, file=cls.LOG_FILE, flush=True)
    
    @classmethod
    def info(cls, message: str, context: Optional[Dict[str, Any]] = None) -> None:
        """Log an info message"""
        if cls._should_log(LogLevel.INFO):
            log_str = cls._format_log(LogLevel.INFO, message, context)
            print(log_str, file=cls.LOG_FILE, flush=True)
    
    @classmethod
    def warning(cls, message: str, context: Optional[Dict[str, Any]] = None) -> None:
        """Log a warning message"""
        if cls._should_log(LogLevel.WARNING):
            log_str = cls._format_log(LogLevel.WARNING, message, context)
            print(log_str, file=cls.LOG_FILE, flush=True)
    
    @classmethod
    def error(cls, message: str, context: Optional[Dict[str, Any]] = None) -> None:
        """Log an error message"""
        if cls._should_log(LogLevel.ERROR):
            log_str = cls._format_log(LogLevel.ERROR, message, context)
            print(log_str, file=cls.LOG_FILE, flush=True)
    
    @classmethod
    def exception(cls, message: str, exc: Optional[Exception] = None, context: Optional[Dict[str, Any]] = None) -> None:
        """
        Log an exception with traceback
        
        Args:
            message: The error message
            exc: The exception object (if None, uses the current exception from sys.exc_info)
            context: Additional context information
        """
        if cls._should_log(LogLevel.ERROR):
            if exc is None:
                exc_info = sys.exc_info()
                if exc_info[0] is not None:
                    exc = exc_info[1]
            
            # Add exception details to context
            ctx = context or {}
            if exc:
                ctx["exception_type"] = exc.__class__.__name__
                ctx["exception_message"] = str(exc)
                ctx["traceback"] = traceback.format_exc()
            
            log_str = cls._format_log(LogLevel.ERROR, f"EXCEPTION: {message}", ctx)
            print(log_str, file=cls.LOG_FILE, flush=True)


class ApiDebugger:
    """
    Utility for debugging API requests and responses.
    
    Provides methods for logging API related information in a structured format.
    """
    
    @staticmethod
    def log_request(method: str, url: str, headers: Dict[str, str], body: Any = None) -> None:
        """
        Log details of an outgoing API request
        
        Args:
            method: HTTP method (GET, POST, etc.)
            url: Request URL
            headers: Request headers (sensitive data like tokens will be masked)
            body: Request body (if any)
        """
        # Clone headers and mask sensitive values
        safe_headers = {}
        for key, value in headers.items():
            if key.lower() in ('authorization', 'api-key'):
                # Mask sensitive headers but show type
                if value.lower().startswith('bearer '):
                    safe_headers[key] = 'Bearer [MASKED_TOKEN]'
                else:
                    safe_headers[key] = '[MASKED_VALUE]'
            else:
                safe_headers[key] = value
        
        # Prepare context for logging
        context = {
            "method": method,
            "url": url,
            "headers": safe_headers
        }
        
        # Add body if present, but be careful with sensitive data
        if body:
            try:
                if isinstance(body, dict):
                    # Deep copy to avoid modifying the original
                    body_copy = json.loads(json.dumps(body))
                    # Mask potential sensitive fields
                    for key in body_copy:
                        if key.lower() in ('password', 'token', 'secret', 'key', 'auth'):
                            body_copy[key] = '[MASKED]'
                    context["body"] = body_copy
                else:
                    # For non-dict bodies, just note the type
                    context["body"] = f"<{type(body).__name__}>"
            except (TypeError, ValueError):
                context["body"] = "<unable to serialize body>"
        
        Logger.debug("API Request", context)
    
    @staticmethod
    def log_response(status_code: int, headers: Dict[str, str], body: Any = None, 
                     elapsed_time: Optional[float] = None) -> None:
        """
        Log details of an API response
        
        Args:
            status_code: HTTP status code
            headers: Response headers
            body: Response body (if any)
            elapsed_time: Time taken for the request in seconds (optional)
        """
        context = {
            "status_code": status_code,
            "headers": dict(headers) if headers else {}
        }
        
        if elapsed_time is not None:
            context["elapsed_time_ms"] = round(elapsed_time * 1000, 2)
        
        # Include body in debug logs, but be careful with potentially large responses
        if body:
            try:
                if isinstance(body, bytes):
                    # For binary responses, just log the size
                    context["body_size_bytes"] = len(body)
                    # Try to decode as UTF-8 if it seems to be text
                    if body.startswith(b'{') or body.startswith(b'['):
                        try:
                            decoded = body.decode('utf-8')
                            parsed = json.loads(decoded)
                            # Truncate large responses to avoid overwhelming logs
                            context["body"] = _truncate_response(parsed)
                        except (UnicodeDecodeError, json.JSONDecodeError):
                            # Not valid UTF-8 JSON
                            pass
                elif isinstance(body, (dict, list)):
                    context["body"] = _truncate_response(body)
                else:
                    context["body"] = str(body)[:1000] + '...' if len(str(body)) > 1000 else str(body)
            except Exception as e:
                context["body_error"] = f"Error processing body: {str(e)}"
        
        log_level = LogLevel.DEBUG
        if status_code >= 400:
            log_level = LogLevel.ERROR
            Logger.error(f"API Response Error {status_code}", context)
        else:
            Logger.debug("API Response", context)


def _truncate_response(data: Union[Dict, List], max_depth: int = 2, 
                      max_items: int = 10, current_depth: int = 0) -> Union[Dict, List, str]:
    """
    Helper function to truncate large API responses for logging
    
    Args:
        data: The data to truncate (dict or list)
        max_depth: Maximum nesting depth to include
        max_items: Maximum items in lists/dicts to include
        current_depth: Current nesting depth (used recursively)
        
    Returns:
        Truncated version of the data
    """
    if current_depth >= max_depth:
        if isinstance(data, (dict, list)):
            return f"<{type(data).__name__} with {len(data)} items>"
        return data
    
    if isinstance(data, dict):
        result = {}
        items = list(data.items())
        for i, (key, value) in enumerate(items):
            if i >= max_items:
                result["..."] = f"<{len(data) - max_items} more items>"
                break
            result[key] = _truncate_response(value, max_depth, max_items, current_depth + 1)
        return result
    
    elif isinstance(data, list):
        result = []
        for i, item in enumerate(data):
            if i >= max_items:
                result.append(f"<{len(data) - max_items} more items>")
                break
            result.append(_truncate_response(item, max_depth, max_items, current_depth + 1))
        return result
    
    return data


def timed_execution(func_name: str = None):
    """
    Decorator to measure and log the execution time of a function
    
    Args:
        func_name: Optional custom name for the function in logs
        
    Returns:
        Decorated function
    """
    def decorator(func):
        def wrapper(*args, **kwargs):
            name = func_name or func.__name__
            start_time = time.time()
            Logger.debug(f"Starting {name}")
            
            try:
                result = func(*args, **kwargs)
                end_time = time.time()
                elapsed = end_time - start_time
                Logger.debug(f"Completed {name} in {elapsed:.3f} seconds")
                return result
            except Exception as e:
                end_time = time.time()
                elapsed = end_time - start_time
                Logger.exception(
                    f"Exception in {name} after {elapsed:.3f} seconds", 
                    exc=e,
                    context={"args": str(args), "kwargs": str(kwargs)}
                )
                raise
                
        return wrapper
    return decorator


# Initialize with default settings
def initialize_logging(level: LogLevel = LogLevel.INFO, log_file=sys.stderr):
    """
    Initialize the logging system with the specified settings
    
    Args:
        level: The log level to use
        log_file: The file handle to write logs to
    """
    Logger.set_log_file(log_file)
    Logger.set_log_level(level)
    Logger.info(f"Logging initialized at level {level.name}") 