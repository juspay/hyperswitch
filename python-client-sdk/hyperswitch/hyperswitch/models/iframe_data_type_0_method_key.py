from enum import Enum


class IframeDataType0MethodKey(str, Enum):
    THREEDSMETHODDATA = "threeDSMethodData"

    def __str__(self) -> str:
        return str(self.value)
