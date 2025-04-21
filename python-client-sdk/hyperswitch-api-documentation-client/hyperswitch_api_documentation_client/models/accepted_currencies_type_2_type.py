from enum import Enum


class AcceptedCurrenciesType2Type(str, Enum):
    ALL_ACCEPTED = "all_accepted"

    def __str__(self) -> str:
        return str(self.value)
