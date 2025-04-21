from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BlikBankRedirectAdditionalData")


@_attrs_define
class BlikBankRedirectAdditionalData:
    """
    Attributes:
        blik_code (Union[None, Unset, str]):  Example: 3GD9MO.
    """

    blik_code: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        blik_code: Union[None, Unset, str]
        if isinstance(self.blik_code, Unset):
            blik_code = UNSET
        else:
            blik_code = self.blik_code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if blik_code is not UNSET:
            field_dict["blik_code"] = blik_code

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_blik_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        blik_code = _parse_blik_code(d.pop("blik_code", UNSET))

        blik_bank_redirect_additional_data = cls(
            blik_code=blik_code,
        )

        blik_bank_redirect_additional_data.additional_properties = d
        return blik_bank_redirect_additional_data

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
