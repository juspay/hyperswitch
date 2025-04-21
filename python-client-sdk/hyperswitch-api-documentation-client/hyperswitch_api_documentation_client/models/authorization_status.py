from enum import Enum


class AuthorizationStatus(str, Enum):
    FAILURE = "failure"
    PROCESSING = "processing"
    SUCCESS = "success"
    UNRESOLVED = "unresolved"

    def __str__(self) -> str:
        return str(self.value)
