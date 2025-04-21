from enum import Enum


class VolumeSplitType(str, Enum):
    VOLUME_SPLIT = "volume_split"

    def __str__(self) -> str:
        return str(self.value)
