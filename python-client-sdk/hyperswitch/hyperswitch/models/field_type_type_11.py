from enum import Enum


class FieldTypeType11(str, Enum):
    USER_CRYPTO_CURRENCY_NETWORK = "user_crypto_currency_network"

    def __str__(self) -> str:
        return str(self.value)
