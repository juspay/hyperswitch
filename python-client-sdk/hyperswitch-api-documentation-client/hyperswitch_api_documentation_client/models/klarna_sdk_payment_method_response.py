from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="KlarnaSdkPaymentMethodResponse")


@_attrs_define
class KlarnaSdkPaymentMethodResponse:
    """
    Attributes:
        payment_type (Union[None, Unset, str]):
    """

    payment_type: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_type: Union[None, Unset, str]
        if isinstance(self.payment_type, Unset):
            payment_type = UNSET
        else:
            payment_type = self.payment_type

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if payment_type is not UNSET:
            field_dict["payment_type"] = payment_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_payment_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_type = _parse_payment_type(d.pop("payment_type", UNSET))

        klarna_sdk_payment_method_response = cls(
            payment_type=payment_type,
        )

        klarna_sdk_payment_method_response.additional_properties = d
        return klarna_sdk_payment_method_response

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
