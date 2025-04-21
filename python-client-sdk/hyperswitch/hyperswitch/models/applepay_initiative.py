from enum import Enum


class ApplepayInitiative(str, Enum):
    IOS = "ios"
    WEB = "web"

    def __str__(self) -> str:
        return str(self.value)
