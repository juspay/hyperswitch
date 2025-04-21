from enum import Enum


class DeviceChannel(str, Enum):
    APP = "APP"
    BRW = "BRW"

    def __str__(self) -> str:
        return str(self.value)
