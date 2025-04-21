from enum import Enum


class PaymentConnectorCategory(str, Enum):
    ALTERNATIVE_PAYMENT_METHOD = "alternative_payment_method"
    BANK_ACQUIRER = "bank_acquirer"
    PAYMENT_GATEWAY = "payment_gateway"

    def __str__(self) -> str:
        return str(self.value)
