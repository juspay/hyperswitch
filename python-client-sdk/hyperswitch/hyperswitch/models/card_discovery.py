from enum import Enum


class CardDiscovery(str, Enum):
    CLICK_TO_PAY = "click_to_pay"
    MANUAL = "manual"
    SAVED_CARD = "saved_card"

    def __str__(self) -> str:
        return str(self.value)
