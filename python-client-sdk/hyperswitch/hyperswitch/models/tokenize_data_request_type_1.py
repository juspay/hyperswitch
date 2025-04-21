from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.tokenize_payment_method_request import TokenizePaymentMethodRequest


T = TypeVar("T", bound="TokenizeDataRequestType1")


@_attrs_define
class TokenizeDataRequestType1:
    """
    Attributes:
        existing_payment_method (TokenizePaymentMethodRequest):
    """

    existing_payment_method: "TokenizePaymentMethodRequest"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        existing_payment_method = self.existing_payment_method.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "existing_payment_method": existing_payment_method,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.tokenize_payment_method_request import TokenizePaymentMethodRequest

        d = dict(src_dict)
        existing_payment_method = TokenizePaymentMethodRequest.from_dict(d.pop("existing_payment_method"))

        tokenize_data_request_type_1 = cls(
            existing_payment_method=existing_payment_method,
        )

        tokenize_data_request_type_1.additional_properties = d
        return tokenize_data_request_type_1

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
