from enum import Enum


class PayoutConnectors(str, Enum):
    ADYEN = "adyen"
    ADYENPLATFORM = "adyenplatform"
    CYBERSOURCE = "cybersource"
    EBANX = "ebanx"
    NOMUPAY = "nomupay"
    PAYONE = "payone"
    PAYPAL = "paypal"
    STRIPE = "stripe"
    WISE = "wise"

    def __str__(self) -> str:
        return str(self.value)
