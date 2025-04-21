from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.capture_status import CaptureStatus
from ..models.currency import Currency
from ..types import UNSET, Unset

T = TypeVar("T", bound="CaptureResponse")


@_attrs_define
class CaptureResponse:
    """
    Attributes:
        capture_id (str): Unique identifier for the capture
        status (CaptureStatus):
        amount (int): The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents
            for USD denomination, in paisa for INR denomination etc., Example: 6540.
        connector (str): The connector used for the payment
        authorized_attempt_id (str): Unique identifier for the parent attempt on which this capture is made
        capture_sequence (int): Sequence number of this capture, in the series of captures made for the parent attempt
        currency (Union[Currency, None, Unset]):
        connector_capture_id (Union[None, Unset, str]): A unique identifier for this capture provided by the connector
        error_message (Union[None, Unset, str]): If there was an error while calling the connector the error message is
            received here
        error_code (Union[None, Unset, str]): If there was an error while calling the connectors the code is received
            here
        error_reason (Union[None, Unset, str]): If there was an error while calling the connectors the reason is
            received here
        reference_id (Union[None, Unset, str]): Reference to the capture at connector side
    """

    capture_id: str
    status: CaptureStatus
    amount: int
    connector: str
    authorized_attempt_id: str
    capture_sequence: int
    currency: Union[Currency, None, Unset] = UNSET
    connector_capture_id: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    error_reason: Union[None, Unset, str] = UNSET
    reference_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        capture_id = self.capture_id

        status = self.status.value

        amount = self.amount

        connector = self.connector

        authorized_attempt_id = self.authorized_attempt_id

        capture_sequence = self.capture_sequence

        currency: Union[None, Unset, str]
        if isinstance(self.currency, Unset):
            currency = UNSET
        elif isinstance(self.currency, Currency):
            currency = self.currency.value
        else:
            currency = self.currency

        connector_capture_id: Union[None, Unset, str]
        if isinstance(self.connector_capture_id, Unset):
            connector_capture_id = UNSET
        else:
            connector_capture_id = self.connector_capture_id

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

        error_reason: Union[None, Unset, str]
        if isinstance(self.error_reason, Unset):
            error_reason = UNSET
        else:
            error_reason = self.error_reason

        reference_id: Union[None, Unset, str]
        if isinstance(self.reference_id, Unset):
            reference_id = UNSET
        else:
            reference_id = self.reference_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "capture_id": capture_id,
                "status": status,
                "amount": amount,
                "connector": connector,
                "authorized_attempt_id": authorized_attempt_id,
                "capture_sequence": capture_sequence,
            }
        )
        if currency is not UNSET:
            field_dict["currency"] = currency
        if connector_capture_id is not UNSET:
            field_dict["connector_capture_id"] = connector_capture_id
        if error_message is not UNSET:
            field_dict["error_message"] = error_message
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if error_reason is not UNSET:
            field_dict["error_reason"] = error_reason
        if reference_id is not UNSET:
            field_dict["reference_id"] = reference_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        capture_id = d.pop("capture_id")

        status = CaptureStatus(d.pop("status"))

        amount = d.pop("amount")

        connector = d.pop("connector")

        authorized_attempt_id = d.pop("authorized_attempt_id")

        capture_sequence = d.pop("capture_sequence")

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

        def _parse_connector_capture_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_capture_id = _parse_connector_capture_id(d.pop("connector_capture_id", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

        def _parse_error_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_reason = _parse_error_reason(d.pop("error_reason", UNSET))

        def _parse_reference_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        reference_id = _parse_reference_id(d.pop("reference_id", UNSET))

        capture_response = cls(
            capture_id=capture_id,
            status=status,
            amount=amount,
            connector=connector,
            authorized_attempt_id=authorized_attempt_id,
            capture_sequence=capture_sequence,
            currency=currency,
            connector_capture_id=connector_capture_id,
            error_message=error_message,
            error_code=error_code,
            error_reason=error_reason,
            reference_id=reference_id,
        )

        capture_response.additional_properties = d
        return capture_response

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
