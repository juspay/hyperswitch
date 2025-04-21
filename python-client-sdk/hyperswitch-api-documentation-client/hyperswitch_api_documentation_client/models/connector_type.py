from enum import Enum


class ConnectorType(str, Enum):
    AUTHENTICATION_PROCESSOR = "authentication_processor"
    BANKING_ENTITIES = "banking_entities"
    BILLING_PROCESSOR = "billing_processor"
    FIN_OPERATIONS = "fin_operations"
    FIZ_OPERATIONS = "fiz_operations"
    NETWORKS = "networks"
    NON_BANKING_FINANCE = "non_banking_finance"
    PAYMENT_METHOD_AUTH = "payment_method_auth"
    PAYMENT_PROCESSOR = "payment_processor"
    PAYMENT_VAS = "payment_vas"
    PAYOUT_PROCESSOR = "payout_processor"
    TAX_PROCESSOR = "tax_processor"

    def __str__(self) -> str:
        return str(self.value)
