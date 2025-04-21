from enum import Enum


class AdyenSplitType(str, Enum):
    ACQUIRINGFEES = "AcquiringFees"
    ADYENCOMMISSION = "AdyenCommission"
    ADYENFEES = "AdyenFees"
    ADYENMARKUP = "AdyenMarkup"
    BALANCEACCOUNT = "BalanceAccount"
    COMMISSION = "Commission"
    INTERCHANGE = "Interchange"
    PAYMENTFEE = "PaymentFee"
    SCHEMEFEE = "SchemeFee"
    TOPUP = "TopUp"
    VAT = "Vat"

    def __str__(self) -> str:
        return str(self.value)
