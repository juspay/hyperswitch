from enum import Enum


class ApplePayAddressParameters(str, Enum):
    EMAIL = "email"
    PHONE = "phone"
    POSTALADDRESS = "postalAddress"

    def __str__(self) -> str:
        return str(self.value)
