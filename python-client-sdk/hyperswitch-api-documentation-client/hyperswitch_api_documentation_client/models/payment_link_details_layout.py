from enum import Enum


class PaymentLinkDetailsLayout(str, Enum):
    LAYOUT1 = "layout1"
    LAYOUT2 = "layout2"

    def __str__(self) -> str:
        return str(self.value)
