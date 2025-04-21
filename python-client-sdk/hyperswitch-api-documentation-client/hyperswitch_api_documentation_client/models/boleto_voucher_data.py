from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BoletoVoucherData")


@_attrs_define
class BoletoVoucherData:
    """
    Attributes:
        social_security_number (Union[None, Unset, str]): The shopper's social security number
    """

    social_security_number: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        social_security_number: Union[None, Unset, str]
        if isinstance(self.social_security_number, Unset):
            social_security_number = UNSET
        else:
            social_security_number = self.social_security_number

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if social_security_number is not UNSET:
            field_dict["social_security_number"] = social_security_number

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_social_security_number(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        social_security_number = _parse_social_security_number(d.pop("social_security_number", UNSET))

        boleto_voucher_data = cls(
            social_security_number=social_security_number,
        )

        boleto_voucher_data.additional_properties = d
        return boleto_voucher_data

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
