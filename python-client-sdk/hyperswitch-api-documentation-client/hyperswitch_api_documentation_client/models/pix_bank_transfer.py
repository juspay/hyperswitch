from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PixBankTransfer")


@_attrs_define
class PixBankTransfer:
    """
    Attributes:
        bank_account_number (str): Bank account number is an unique identifier assigned by a bank to a customer.
            Example: 000123456.
        pix_key (str): Unique key for pix customer Example: 000123456.
        bank_name (Union[None, Unset, str]): Bank name Example: Deutsche Bank.
        bank_branch (Union[None, Unset, str]): Bank branch Example: 3707.
        tax_id (Union[None, Unset, str]): Individual taxpayer identification number Example: 000123456.
    """

    bank_account_number: str
    pix_key: str
    bank_name: Union[None, Unset, str] = UNSET
    bank_branch: Union[None, Unset, str] = UNSET
    tax_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_account_number = self.bank_account_number

        pix_key = self.pix_key

        bank_name: Union[None, Unset, str]
        if isinstance(self.bank_name, Unset):
            bank_name = UNSET
        else:
            bank_name = self.bank_name

        bank_branch: Union[None, Unset, str]
        if isinstance(self.bank_branch, Unset):
            bank_branch = UNSET
        else:
            bank_branch = self.bank_branch

        tax_id: Union[None, Unset, str]
        if isinstance(self.tax_id, Unset):
            tax_id = UNSET
        else:
            tax_id = self.tax_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank_account_number": bank_account_number,
                "pix_key": pix_key,
            }
        )
        if bank_name is not UNSET:
            field_dict["bank_name"] = bank_name
        if bank_branch is not UNSET:
            field_dict["bank_branch"] = bank_branch
        if tax_id is not UNSET:
            field_dict["tax_id"] = tax_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        bank_account_number = d.pop("bank_account_number")

        pix_key = d.pop("pix_key")

        def _parse_bank_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_name = _parse_bank_name(d.pop("bank_name", UNSET))

        def _parse_bank_branch(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_branch = _parse_bank_branch(d.pop("bank_branch", UNSET))

        def _parse_tax_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        tax_id = _parse_tax_id(d.pop("tax_id", UNSET))

        pix_bank_transfer = cls(
            bank_account_number=bank_account_number,
            pix_key=pix_key,
            bank_name=bank_name,
            bank_branch=bank_branch,
            tax_id=tax_id,
        )

        pix_bank_transfer.additional_properties = d
        return pix_bank_transfer

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
