from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.bank_names import BankNames
from ..types import UNSET, Unset

T = TypeVar("T", bound="BankRedirectResponse")


@_attrs_define
class BankRedirectResponse:
    """
    Attributes:
        bank_name (Union[BankNames, None, Unset]):
    """

    bank_name: Union[BankNames, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_name: Union[None, Unset, str]
        if isinstance(self.bank_name, Unset):
            bank_name = UNSET
        elif isinstance(self.bank_name, BankNames):
            bank_name = self.bank_name.value
        else:
            bank_name = self.bank_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if bank_name is not UNSET:
            field_dict["bank_name"] = bank_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_bank_name(data: object) -> Union[BankNames, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                bank_name_type_1 = BankNames(data)

                return bank_name_type_1
            except:  # noqa: E722
                pass
            return cast(Union[BankNames, None, Unset], data)

        bank_name = _parse_bank_name(d.pop("bank_name", UNSET))

        bank_redirect_response = cls(
            bank_name=bank_name,
        )

        bank_redirect_response.additional_properties = d
        return bank_redirect_response

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
