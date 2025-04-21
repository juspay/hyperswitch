from enum import Enum


class ThreeDsMethodDataType0ThreeDsMethodKey(str, Enum):
    THREEDSMETHODDATA = "threeDSMethodData"

    def __str__(self) -> str:
        return str(self.value)
