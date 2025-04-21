from enum import Enum


class AttemptStatus(str, Enum):
    AUTHENTICATION_FAILED = "authentication_failed"
    AUTHENTICATION_PENDING = "authentication_pending"
    AUTHENTICATION_SUCCESSFUL = "authentication_successful"
    AUTHORIZATION_FAILED = "authorization_failed"
    AUTHORIZED = "authorized"
    AUTHORIZING = "authorizing"
    AUTO_REFUNDED = "auto_refunded"
    CAPTURE_FAILED = "capture_failed"
    CAPTURE_INITIATED = "capture_initiated"
    CHARGED = "charged"
    COD_INITIATED = "cod_initiated"
    CONFIRMATION_AWAITED = "confirmation_awaited"
    DEVICE_DATA_COLLECTION_PENDING = "device_data_collection_pending"
    FAILURE = "failure"
    PARTIAL_CHARGED = "partial_charged"
    PARTIAL_CHARGED_AND_CHARGEABLE = "partial_charged_and_chargeable"
    PAYMENT_METHOD_AWAITED = "payment_method_awaited"
    PENDING = "pending"
    ROUTER_DECLINED = "router_declined"
    STARTED = "started"
    UNRESOLVED = "unresolved"
    VOIDED = "voided"
    VOID_FAILED = "void_failed"
    VOID_INITIATED = "void_initiated"

    def __str__(self) -> str:
        return str(self.value)
