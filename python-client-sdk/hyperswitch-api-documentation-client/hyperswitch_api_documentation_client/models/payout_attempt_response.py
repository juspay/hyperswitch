from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.payment_method_type import PaymentMethodType
from ..models.payout_status import PayoutStatus
from ..models.payout_type import PayoutType
from ..types import UNSET, Unset

T = TypeVar("T", bound="PayoutAttemptResponse")


@_attrs_define
class PayoutAttemptResponse:
    """
    Attributes:
        attempt_id (str): Unique identifier for the attempt
        status (PayoutStatus):
        amount (int): The payout attempt amount. Amount for the payout in lowest denomination of the currency. (i.e) in
            cents for USD denomination, in paisa for INR denomination etc., Example: 6583.
        currency (Union[Currency, None, Unset]):
        connector (Union[None, Unset, str]): The connector used for the payout
        error_code (Union[None, Unset, str]): Connector's error code in case of failures
        error_message (Union[None, Unset, str]): Connector's error message in case of failures
        payment_method (Union[None, PayoutType, Unset]):
        payout_method_type (Union[None, PaymentMethodType, Unset]):
        connector_transaction_id (Union[None, Unset, str]): A unique identifier for a payout provided by the connector
        cancellation_reason (Union[None, Unset, str]): If the payout was cancelled the reason provided here
        unified_code (Union[None, Unset, str]): (This field is not live yet)
            Error code unified across the connectors is received here in case of errors while calling the underlying
            connector Example: UE_000.
        unified_message (Union[None, Unset, str]): (This field is not live yet)
            Error message unified across the connectors is received here in case of errors while calling the underlying
            connector Example: Invalid card details.
    """

    attempt_id: str
    status: PayoutStatus
    amount: int
    currency: Union[Currency, None, Unset] = UNSET
    connector: Union[None, Unset, str] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    payment_method: Union[None, PayoutType, Unset] = UNSET
    payout_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    connector_transaction_id: Union[None, Unset, str] = UNSET
    cancellation_reason: Union[None, Unset, str] = UNSET
    unified_code: Union[None, Unset, str] = UNSET
    unified_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempt_id = self.attempt_id

        status = self.status.value

        amount = self.amount

        currency: Union[None, Unset, str]
        if isinstance(self.currency, Unset):
            currency = UNSET
        elif isinstance(self.currency, Currency):
            currency = self.currency.value
        else:
            currency = self.currency

        connector: Union[None, Unset, str]
        if isinstance(self.connector, Unset):
            connector = UNSET
        else:
            connector = self.connector

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        payment_method: Union[None, Unset, str]
        if isinstance(self.payment_method, Unset):
            payment_method = UNSET
        elif isinstance(self.payment_method, PayoutType):
            payment_method = self.payment_method.value
        else:
            payment_method = self.payment_method

        payout_method_type: Union[None, Unset, str]
        if isinstance(self.payout_method_type, Unset):
            payout_method_type = UNSET
        elif isinstance(self.payout_method_type, PaymentMethodType):
            payout_method_type = self.payout_method_type.value
        else:
            payout_method_type = self.payout_method_type

        connector_transaction_id: Union[None, Unset, str]
        if isinstance(self.connector_transaction_id, Unset):
            connector_transaction_id = UNSET
        else:
            connector_transaction_id = self.connector_transaction_id

        cancellation_reason: Union[None, Unset, str]
        if isinstance(self.cancellation_reason, Unset):
            cancellation_reason = UNSET
        else:
            cancellation_reason = self.cancellation_reason

        unified_code: Union[None, Unset, str]
        if isinstance(self.unified_code, Unset):
            unified_code = UNSET
        else:
            unified_code = self.unified_code

        unified_message: Union[None, Unset, str]
        if isinstance(self.unified_message, Unset):
            unified_message = UNSET
        else:
            unified_message = self.unified_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempt_id": attempt_id,
                "status": status,
                "amount": amount,
            }
        )
        if currency is not UNSET:
            field_dict["currency"] = currency
        if connector is not UNSET:
            field_dict["connector"] = connector
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if error_message is not UNSET:
            field_dict["error_message"] = error_message
        if payment_method is not UNSET:
            field_dict["payment_method"] = payment_method
        if payout_method_type is not UNSET:
            field_dict["payout_method_type"] = payout_method_type
        if connector_transaction_id is not UNSET:
            field_dict["connector_transaction_id"] = connector_transaction_id
        if cancellation_reason is not UNSET:
            field_dict["cancellation_reason"] = cancellation_reason
        if unified_code is not UNSET:
            field_dict["unified_code"] = unified_code
        if unified_message is not UNSET:
            field_dict["unified_message"] = unified_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        attempt_id = d.pop("attempt_id")

        status = PayoutStatus(d.pop("status"))

        amount = d.pop("amount")

        def _parse_currency(data: object) -> Union[Currency, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                currency_type_1 = Currency(data)

                return currency_type_1
            except:  # noqa: E722
                pass
            return cast(Union[Currency, None, Unset], data)

        currency = _parse_currency(d.pop("currency", UNSET))

        def _parse_connector(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector = _parse_connector(d.pop("connector", UNSET))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        def _parse_payment_method(data: object) -> Union[None, PayoutType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_1 = PayoutType(data)

                return payment_method_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PayoutType, Unset], data)

        payment_method = _parse_payment_method(d.pop("payment_method", UNSET))

        def _parse_payout_method_type(data: object) -> Union[None, PaymentMethodType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payout_method_type_type_1 = PaymentMethodType(data)

                return payout_method_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodType, Unset], data)

        payout_method_type = _parse_payout_method_type(d.pop("payout_method_type", UNSET))

        def _parse_connector_transaction_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_transaction_id = _parse_connector_transaction_id(d.pop("connector_transaction_id", UNSET))

        def _parse_cancellation_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        cancellation_reason = _parse_cancellation_reason(d.pop("cancellation_reason", UNSET))

        def _parse_unified_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        unified_code = _parse_unified_code(d.pop("unified_code", UNSET))

        def _parse_unified_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        unified_message = _parse_unified_message(d.pop("unified_message", UNSET))

        payout_attempt_response = cls(
            attempt_id=attempt_id,
            status=status,
            amount=amount,
            currency=currency,
            connector=connector,
            error_code=error_code,
            error_message=error_message,
            payment_method=payment_method,
            payout_method_type=payout_method_type,
            connector_transaction_id=connector_transaction_id,
            cancellation_reason=cancellation_reason,
            unified_code=unified_code,
            unified_message=unified_message,
        )

        payout_attempt_response.additional_properties = d
        return payout_attempt_response

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
