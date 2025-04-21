import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.attempt_status import AttemptStatus
from ..models.authentication_type import AuthenticationType
from ..models.capture_method import CaptureMethod
from ..models.currency import Currency
from ..models.payment_experience import PaymentExperience
from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

T = TypeVar("T", bound="PaymentAttemptResponse")


@_attrs_define
class PaymentAttemptResponse:
    """
    Attributes:
        attempt_id (str): Unique identifier for the attempt
        status (AttemptStatus): The status of the attempt
        amount (int): The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e)
            in cents for USD denomination, in paisa for INR denomination etc., Example: 6540.
        created_at (datetime.datetime): Time at which the payment attempt was created Example: 2022-09-10T10:11:12Z.
        modified_at (datetime.datetime): Time at which the payment attempt was last modified Example:
            2022-09-10T10:11:12Z.
        order_tax_amount (Union[None, Unset, int]): The payment attempt tax_amount. Example: 6540.
        currency (Union[Currency, None, Unset]):
        connector (Union[None, Unset, str]): The connector used for the payment
        error_message (Union[None, Unset, str]): If there was an error while calling the connector, the error message is
            received here
        payment_method (Union[None, PaymentMethod, Unset]):
        connector_transaction_id (Union[None, Unset, str]): A unique identifier for a payment provided by the connector
        capture_method (Union[CaptureMethod, None, Unset]):
        authentication_type (Union[AuthenticationType, None, Unset]):  Default: AuthenticationType.THREE_DS.
        cancellation_reason (Union[None, Unset, str]): If the payment was cancelled the reason will be provided here
        mandate_id (Union[None, Unset, str]): A unique identifier to link the payment to a mandate, can be use instead
            of payment_method_data
        error_code (Union[None, Unset, str]): If there was an error while calling the connectors the error code is
            received here
        payment_token (Union[None, Unset, str]): Provide a reference to a stored payment method
        connector_metadata (Union[Unset, Any]): Additional data related to some connectors
        payment_experience (Union[None, PaymentExperience, Unset]):
        payment_method_type (Union[None, PaymentMethodType, Unset]):
        reference_id (Union[None, Unset, str]): Reference to the payment at connector side Example: 993672945374576J.
        unified_code (Union[None, Unset, str]): (This field is not live yet)Error code unified across the connectors is
            received here if there was an error while calling connector
        unified_message (Union[None, Unset, str]): (This field is not live yet)Error message unified across the
            connectors is received here if there was an error while calling connector
        client_source (Union[None, Unset, str]): Value passed in X-CLIENT-SOURCE header during payments confirm request
            by the client
        client_version (Union[None, Unset, str]): Value passed in X-CLIENT-VERSION header during payments confirm
            request by the client
    """

    attempt_id: str
    status: AttemptStatus
    amount: int
    created_at: datetime.datetime
    modified_at: datetime.datetime
    order_tax_amount: Union[None, Unset, int] = UNSET
    currency: Union[Currency, None, Unset] = UNSET
    connector: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    payment_method: Union[None, PaymentMethod, Unset] = UNSET
    connector_transaction_id: Union[None, Unset, str] = UNSET
    capture_method: Union[CaptureMethod, None, Unset] = UNSET
    authentication_type: Union[AuthenticationType, None, Unset] = AuthenticationType.THREE_DS
    cancellation_reason: Union[None, Unset, str] = UNSET
    mandate_id: Union[None, Unset, str] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    payment_token: Union[None, Unset, str] = UNSET
    connector_metadata: Union[Unset, Any] = UNSET
    payment_experience: Union[None, PaymentExperience, Unset] = UNSET
    payment_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    reference_id: Union[None, Unset, str] = UNSET
    unified_code: Union[None, Unset, str] = UNSET
    unified_message: Union[None, Unset, str] = UNSET
    client_source: Union[None, Unset, str] = UNSET
    client_version: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempt_id = self.attempt_id

        status = self.status.value

        amount = self.amount

        created_at = self.created_at.isoformat()

        modified_at = self.modified_at.isoformat()

        order_tax_amount: Union[None, Unset, int]
        if isinstance(self.order_tax_amount, Unset):
            order_tax_amount = UNSET
        else:
            order_tax_amount = self.order_tax_amount

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

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        payment_method: Union[None, Unset, str]
        if isinstance(self.payment_method, Unset):
            payment_method = UNSET
        elif isinstance(self.payment_method, PaymentMethod):
            payment_method = self.payment_method.value
        else:
            payment_method = self.payment_method

        connector_transaction_id: Union[None, Unset, str]
        if isinstance(self.connector_transaction_id, Unset):
            connector_transaction_id = UNSET
        else:
            connector_transaction_id = self.connector_transaction_id

        capture_method: Union[None, Unset, str]
        if isinstance(self.capture_method, Unset):
            capture_method = UNSET
        elif isinstance(self.capture_method, CaptureMethod):
            capture_method = self.capture_method.value
        else:
            capture_method = self.capture_method

        authentication_type: Union[None, Unset, str]
        if isinstance(self.authentication_type, Unset):
            authentication_type = UNSET
        elif isinstance(self.authentication_type, AuthenticationType):
            authentication_type = self.authentication_type.value
        else:
            authentication_type = self.authentication_type

        cancellation_reason: Union[None, Unset, str]
        if isinstance(self.cancellation_reason, Unset):
            cancellation_reason = UNSET
        else:
            cancellation_reason = self.cancellation_reason

        mandate_id: Union[None, Unset, str]
        if isinstance(self.mandate_id, Unset):
            mandate_id = UNSET
        else:
            mandate_id = self.mandate_id

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

        payment_token: Union[None, Unset, str]
        if isinstance(self.payment_token, Unset):
            payment_token = UNSET
        else:
            payment_token = self.payment_token

        connector_metadata = self.connector_metadata

        payment_experience: Union[None, Unset, str]
        if isinstance(self.payment_experience, Unset):
            payment_experience = UNSET
        elif isinstance(self.payment_experience, PaymentExperience):
            payment_experience = self.payment_experience.value
        else:
            payment_experience = self.payment_experience

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        elif isinstance(self.payment_method_type, PaymentMethodType):
            payment_method_type = self.payment_method_type.value
        else:
            payment_method_type = self.payment_method_type

        reference_id: Union[None, Unset, str]
        if isinstance(self.reference_id, Unset):
            reference_id = UNSET
        else:
            reference_id = self.reference_id

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

        client_source: Union[None, Unset, str]
        if isinstance(self.client_source, Unset):
            client_source = UNSET
        else:
            client_source = self.client_source

        client_version: Union[None, Unset, str]
        if isinstance(self.client_version, Unset):
            client_version = UNSET
        else:
            client_version = self.client_version

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempt_id": attempt_id,
                "status": status,
                "amount": amount,
                "created_at": created_at,
                "modified_at": modified_at,
            }
        )
        if order_tax_amount is not UNSET:
            field_dict["order_tax_amount"] = order_tax_amount
        if currency is not UNSET:
            field_dict["currency"] = currency
        if connector is not UNSET:
            field_dict["connector"] = connector
        if error_message is not UNSET:
            field_dict["error_message"] = error_message
        if payment_method is not UNSET:
            field_dict["payment_method"] = payment_method
        if connector_transaction_id is not UNSET:
            field_dict["connector_transaction_id"] = connector_transaction_id
        if capture_method is not UNSET:
            field_dict["capture_method"] = capture_method
        if authentication_type is not UNSET:
            field_dict["authentication_type"] = authentication_type
        if cancellation_reason is not UNSET:
            field_dict["cancellation_reason"] = cancellation_reason
        if mandate_id is not UNSET:
            field_dict["mandate_id"] = mandate_id
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if payment_token is not UNSET:
            field_dict["payment_token"] = payment_token
        if connector_metadata is not UNSET:
            field_dict["connector_metadata"] = connector_metadata
        if payment_experience is not UNSET:
            field_dict["payment_experience"] = payment_experience
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type
        if reference_id is not UNSET:
            field_dict["reference_id"] = reference_id
        if unified_code is not UNSET:
            field_dict["unified_code"] = unified_code
        if unified_message is not UNSET:
            field_dict["unified_message"] = unified_message
        if client_source is not UNSET:
            field_dict["client_source"] = client_source
        if client_version is not UNSET:
            field_dict["client_version"] = client_version

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        attempt_id = d.pop("attempt_id")

        status = AttemptStatus(d.pop("status"))

        amount = d.pop("amount")

        created_at = isoparse(d.pop("created_at"))

        modified_at = isoparse(d.pop("modified_at"))

        def _parse_order_tax_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        order_tax_amount = _parse_order_tax_amount(d.pop("order_tax_amount", UNSET))

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

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        def _parse_payment_method(data: object) -> Union[None, PaymentMethod, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_1 = PaymentMethod(data)

                return payment_method_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethod, Unset], data)

        payment_method = _parse_payment_method(d.pop("payment_method", UNSET))

        def _parse_connector_transaction_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_transaction_id = _parse_connector_transaction_id(d.pop("connector_transaction_id", UNSET))

        def _parse_capture_method(data: object) -> Union[CaptureMethod, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                capture_method_type_1 = CaptureMethod(data)

                return capture_method_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CaptureMethod, None, Unset], data)

        capture_method = _parse_capture_method(d.pop("capture_method", UNSET))

        def _parse_authentication_type(data: object) -> Union[AuthenticationType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                authentication_type_type_1 = AuthenticationType(data)

                return authentication_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[AuthenticationType, None, Unset], data)

        authentication_type = _parse_authentication_type(d.pop("authentication_type", UNSET))

        def _parse_cancellation_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        cancellation_reason = _parse_cancellation_reason(d.pop("cancellation_reason", UNSET))

        def _parse_mandate_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        mandate_id = _parse_mandate_id(d.pop("mandate_id", UNSET))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

        def _parse_payment_token(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_token = _parse_payment_token(d.pop("payment_token", UNSET))

        connector_metadata = d.pop("connector_metadata", UNSET)

        def _parse_payment_experience(data: object) -> Union[None, PaymentExperience, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_experience_type_1 = PaymentExperience(data)

                return payment_experience_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentExperience, Unset], data)

        payment_experience = _parse_payment_experience(d.pop("payment_experience", UNSET))

        def _parse_payment_method_type(data: object) -> Union[None, PaymentMethodType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_type_1 = PaymentMethodType(data)

                return payment_method_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodType, Unset], data)

        payment_method_type = _parse_payment_method_type(d.pop("payment_method_type", UNSET))

        def _parse_reference_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        reference_id = _parse_reference_id(d.pop("reference_id", UNSET))

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

        def _parse_client_source(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_source = _parse_client_source(d.pop("client_source", UNSET))

        def _parse_client_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_version = _parse_client_version(d.pop("client_version", UNSET))

        payment_attempt_response = cls(
            attempt_id=attempt_id,
            status=status,
            amount=amount,
            created_at=created_at,
            modified_at=modified_at,
            order_tax_amount=order_tax_amount,
            currency=currency,
            connector=connector,
            error_message=error_message,
            payment_method=payment_method,
            connector_transaction_id=connector_transaction_id,
            capture_method=capture_method,
            authentication_type=authentication_type,
            cancellation_reason=cancellation_reason,
            mandate_id=mandate_id,
            error_code=error_code,
            payment_token=payment_token,
            connector_metadata=connector_metadata,
            payment_experience=payment_experience,
            payment_method_type=payment_method_type,
            reference_id=reference_id,
            unified_code=unified_code,
            unified_message=unified_message,
            client_source=client_source,
            client_version=client_version,
        )

        payment_attempt_response.additional_properties = d
        return payment_attempt_response

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
