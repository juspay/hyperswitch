from enum import Enum


class CaptureMethod(str, Enum):
    AUTOMATIC = "automatic"
    MANUAL = "manual"
    MANUAL_MULTIPLE = "manual_multiple"
    SCHEDULED = "scheduled"
    SEQUENTIAL_AUTOMATIC = "sequential_automatic"

    def __str__(self) -> str:
        return str(self.value)
