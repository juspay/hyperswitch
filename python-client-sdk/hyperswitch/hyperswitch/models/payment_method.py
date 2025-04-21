from enum import Enum


class PaymentMethod(str, Enum):
    BANK_DEBIT = "bank_debit"
    BANK_REDIRECT = "bank_redirect"
    BANK_TRANSFER = "bank_transfer"
    CARD = "card"
    CARD_REDIRECT = "card_redirect"
    CRYPTO = "crypto"
    GIFT_CARD = "gift_card"
    MOBILE_PAYMENT = "mobile_payment"
    OPEN_BANKING = "open_banking"
    PAY_LATER = "pay_later"
    REAL_TIME_PAYMENT = "real_time_payment"
    REWARD = "reward"
    UPI = "upi"
    VOUCHER = "voucher"
    WALLET = "wallet"

    def __str__(self) -> str:
        return str(self.value)
