from enum import Enum


class AuthenticationConnectors(str, Enum):
    CTP_MASTERCARD = "ctp_mastercard"
    CTP_VISA = "ctp_visa"
    GPAYMENTS = "gpayments"
    JUSPAYTHREEDSSERVER = "juspaythreedsserver"
    NETCETERA = "netcetera"
    THREEDSECUREIO = "threedsecureio"
    UNIFIED_AUTHENTICATION_SERVICE = "unified_authentication_service"

    def __str__(self) -> str:
        return str(self.value)
