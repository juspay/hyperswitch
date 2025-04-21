from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.customer_details import CustomerDetails
    from ..models.payment_method_response import PaymentMethodResponse
    from ..models.tokenize_data_request_type_0 import TokenizeDataRequestType0
    from ..models.tokenize_data_request_type_1 import TokenizeDataRequestType1


T = TypeVar("T", bound="CardNetworkTokenizeResponse")


@_attrs_define
class CardNetworkTokenizeResponse:
    """
    Attributes:
        customer (CustomerDetails): Passing this object creates a new customer or attaches an existing customer to the
            payment
        card_tokenized (bool): Card network tokenization status
        payment_method_response (Union['PaymentMethodResponse', None, Unset]):
        error_code (Union[None, Unset, str]): Error code
        error_message (Union[None, Unset, str]): Error message
        tokenization_data (Union['TokenizeDataRequestType0', 'TokenizeDataRequestType1', None, Unset]):
    """

    customer: "CustomerDetails"
    card_tokenized: bool
    payment_method_response: Union["PaymentMethodResponse", None, Unset] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    tokenization_data: Union["TokenizeDataRequestType0", "TokenizeDataRequestType1", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.payment_method_response import PaymentMethodResponse
        from ..models.tokenize_data_request_type_0 import TokenizeDataRequestType0
        from ..models.tokenize_data_request_type_1 import TokenizeDataRequestType1

        customer = self.customer.to_dict()

        card_tokenized = self.card_tokenized

        payment_method_response: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_method_response, Unset):
            payment_method_response = UNSET
        elif isinstance(self.payment_method_response, PaymentMethodResponse):
            payment_method_response = self.payment_method_response.to_dict()
        else:
            payment_method_response = self.payment_method_response

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

        tokenization_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.tokenization_data, Unset):
            tokenization_data = UNSET
        elif isinstance(self.tokenization_data, TokenizeDataRequestType0):
            tokenization_data = self.tokenization_data.to_dict()
        elif isinstance(self.tokenization_data, TokenizeDataRequestType1):
            tokenization_data = self.tokenization_data.to_dict()
        else:
            tokenization_data = self.tokenization_data

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer": customer,
                "card_tokenized": card_tokenized,
            }
        )
        if payment_method_response is not UNSET:
            field_dict["payment_method_response"] = payment_method_response
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if error_message is not UNSET:
            field_dict["error_message"] = error_message
        if tokenization_data is not UNSET:
            field_dict["tokenization_data"] = tokenization_data

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.customer_details import CustomerDetails
        from ..models.payment_method_response import PaymentMethodResponse
        from ..models.tokenize_data_request_type_0 import TokenizeDataRequestType0
        from ..models.tokenize_data_request_type_1 import TokenizeDataRequestType1

        d = dict(src_dict)
        customer = CustomerDetails.from_dict(d.pop("customer"))

        card_tokenized = d.pop("card_tokenized")

        def _parse_payment_method_response(data: object) -> Union["PaymentMethodResponse", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_method_response_type_1 = PaymentMethodResponse.from_dict(data)

                return payment_method_response_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PaymentMethodResponse", None, Unset], data)

        payment_method_response = _parse_payment_method_response(d.pop("payment_method_response", UNSET))

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

        def _parse_tokenization_data(
            data: object,
        ) -> Union["TokenizeDataRequestType0", "TokenizeDataRequestType1", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_tokenize_data_request_type_0 = TokenizeDataRequestType0.from_dict(data)

                return componentsschemas_tokenize_data_request_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_tokenize_data_request_type_1 = TokenizeDataRequestType1.from_dict(data)

                return componentsschemas_tokenize_data_request_type_1
            except:  # noqa: E722
                pass
            return cast(Union["TokenizeDataRequestType0", "TokenizeDataRequestType1", None, Unset], data)

        tokenization_data = _parse_tokenization_data(d.pop("tokenization_data", UNSET))

        card_network_tokenize_response = cls(
            customer=customer,
            card_tokenized=card_tokenized,
            payment_method_response=payment_method_response,
            error_code=error_code,
            error_message=error_message,
            tokenization_data=tokenization_data,
        )

        card_network_tokenize_response.additional_properties = d
        return card_network_tokenize_response

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
