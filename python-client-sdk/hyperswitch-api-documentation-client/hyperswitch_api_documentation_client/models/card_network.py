from enum import Enum


class CardNetwork(str, Enum):
    AMERICANEXPRESS = "AmericanExpress"
    CARTESBANCAIRES = "CartesBancaires"
    DINERSCLUB = "DinersClub"
    DISCOVER = "Discover"
    INTERAC = "Interac"
    JCB = "JCB"
    MAESTRO = "Maestro"
    MASTERCARD = "Mastercard"
    RUPAY = "RuPay"
    UNIONPAY = "UnionPay"
    VISA = "Visa"

    def __str__(self) -> str:
        return str(self.value)
