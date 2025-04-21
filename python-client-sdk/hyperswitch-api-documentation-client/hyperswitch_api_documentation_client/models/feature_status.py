from enum import Enum


class FeatureStatus(str, Enum):
    NOT_SUPPORTED = "not_supported"
    SUPPORTED = "supported"

    def __str__(self) -> str:
        return str(self.value)
