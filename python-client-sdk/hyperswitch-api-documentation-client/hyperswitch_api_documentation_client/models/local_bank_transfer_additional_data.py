from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="LocalBankTransferAdditionalData")


@_attrs_define
class LocalBankTransferAdditionalData:
    """
    Attributes:
        bank_code (Union[None, Unset, str]): Partially masked bank code Example: **** OA2312.
    """

    bank_code: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_code: Union[None, Unset, str]
        if isinstance(self.bank_code, Unset):
            bank_code = UNSET
        else:
            bank_code = self.bank_code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if bank_code is not UNSET:
            field_dict["bank_code"] = bank_code

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_bank_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_code = _parse_bank_code(d.pop("bank_code", UNSET))

        local_bank_transfer_additional_data = cls(
            bank_code=bank_code,
        )

        local_bank_transfer_additional_data.additional_properties = d
        return local_bank_transfer_additional_data

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
