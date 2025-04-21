from enum import Enum


class DynamicRoutingConfigParams(str, Enum):
    AUTHENTICATIONTYPE = "AuthenticationType"
    CARDBIN = "CardBin"
    CARDNETWORK = "CardNetwork"
    COUNTRY = "Country"
    CURRENCY = "Currency"
    PAYMENTMETHOD = "PaymentMethod"
    PAYMENTMETHODTYPE = "PaymentMethodType"

    def __str__(self) -> str:
        return str(self.value)
