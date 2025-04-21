from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="BacsBankTransfer")


@_attrs_define
class BacsBankTransfer:
    """
    Attributes:
        bank_account_number (str): Bank account number is an unique identifier assigned by a bank to a customer.
            Example: 000123456.
        bank_sort_code (str): [6 digits] Sort Code - used in UK and Ireland for identifying a bank and it's branches.
            Example: 98-76-54.
        bank_name (Union[None, Unset, str]): Bank name Example: Deutsche Bank.
        bank_country_code (Union[CountryAlpha2, None, Unset]):
        bank_city (Union[None, Unset, str]): Bank city Example: California.
    """

    bank_account_number: str
    bank_sort_code: str
    bank_name: Union[None, Unset, str] = UNSET
    bank_country_code: Union[CountryAlpha2, None, Unset] = UNSET
    bank_city: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_account_number = self.bank_account_number

        bank_sort_code = self.bank_sort_code

        bank_name: Union[None, Unset, str]
        if isinstance(self.bank_name, Unset):
            bank_name = UNSET
        else:
            bank_name = self.bank_name

        bank_country_code: Union[None, Unset, str]
        if isinstance(self.bank_country_code, Unset):
            bank_country_code = UNSET
        elif isinstance(self.bank_country_code, CountryAlpha2):
            bank_country_code = self.bank_country_code.value
        else:
            bank_country_code = self.bank_country_code

        bank_city: Union[None, Unset, str]
        if isinstance(self.bank_city, Unset):
            bank_city = UNSET
        else:
            bank_city = self.bank_city

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank_account_number": bank_account_number,
                "bank_sort_code": bank_sort_code,
            }
        )
        if bank_name is not UNSET:
            field_dict["bank_name"] = bank_name
        if bank_country_code is not UNSET:
            field_dict["bank_country_code"] = bank_country_code
        if bank_city is not UNSET:
            field_dict["bank_city"] = bank_city

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        bank_account_number = d.pop("bank_account_number")

        bank_sort_code = d.pop("bank_sort_code")

        def _parse_bank_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_name = _parse_bank_name(d.pop("bank_name", UNSET))

        def _parse_bank_country_code(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                bank_country_code_type_1 = CountryAlpha2(data)

                return bank_country_code_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        bank_country_code = _parse_bank_country_code(d.pop("bank_country_code", UNSET))

        def _parse_bank_city(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_city = _parse_bank_city(d.pop("bank_city", UNSET))

        bacs_bank_transfer = cls(
            bank_account_number=bank_account_number,
            bank_sort_code=bank_sort_code,
            bank_name=bank_name,
            bank_country_code=bank_country_code,
            bank_city=bank_city,
        )

        bacs_bank_transfer.additional_properties = d
        return bacs_bank_transfer

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
