from enum import Enum


class AuthenticationStatus(str, Enum):
    FAILED = "failed"
    PENDING = "pending"
    STARTED = "started"
    SUCCESS = "success"

    def __str__(self) -> str:
        return str(self.value)
