from enum import Enum


class FutureUsage(str, Enum):
    OFF_SESSION = "off_session"
    ON_SESSION = "on_session"

    def __str__(self) -> str:
        return str(self.value)
