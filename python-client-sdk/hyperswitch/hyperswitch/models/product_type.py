from enum import Enum


class ProductType(str, Enum):
    ACCOMMODATION = "accommodation"
    DIGITAL = "digital"
    EVENT = "event"
    PHYSICAL = "physical"
    RIDE = "ride"
    TRAVEL = "travel"

    def __str__(self) -> str:
        return str(self.value)
