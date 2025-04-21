from enum import Enum


class AcceptedCountriesType0Type(str, Enum):
    ENABLE_ONLY = "enable_only"

    def __str__(self) -> str:
        return str(self.value)
