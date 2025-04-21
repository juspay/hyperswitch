from enum import Enum


class SdkType(str, Enum):
    VALUE_0 = "01"
    VALUE_1 = "02"
    VALUE_2 = "03"
    VALUE_3 = "04"
    VALUE_4 = "05"

    def __str__(self) -> str:
        return str(self.value)
