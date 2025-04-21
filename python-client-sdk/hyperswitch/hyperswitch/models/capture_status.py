from enum import Enum


class CaptureStatus(str, Enum):
    CHARGED = "charged"
    FAILED = "failed"
    PENDING = "pending"
    STARTED = "started"

    def __str__(self) -> str:
        return str(self.value)
