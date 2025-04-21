from enum import Enum


class StripeChargeType(str, Enum):
    DESTINATION = "destination"
    DIRECT = "direct"

    def __str__(self) -> str:
        return str(self.value)
