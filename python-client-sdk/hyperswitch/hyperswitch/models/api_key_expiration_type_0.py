from enum import Enum


class ApiKeyExpirationType0(str, Enum):
    NEVER = "never"

    def __str__(self) -> str:
        return str(self.value)
