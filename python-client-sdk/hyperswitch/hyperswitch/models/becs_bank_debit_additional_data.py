from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BecsBankDebitAdditionalData")


@_attrs_define
class BecsBankDebitAdditionalData:
    """
    Attributes:
        account_number (str): Partially masked account number for Becs payment method Example: 0001****3456.
        bsb_number (str): Bank-State-Branch (bsb) number Example: 000000.
        bank_account_holder_name (Union[None, Unset, str]): Bank account's owner name Example: John Doe.
    """

    account_number: str
    bsb_number: str
    bank_account_holder_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_number = self.account_number

        bsb_number = self.bsb_number

        bank_account_holder_name: Union[None, Unset, str]
        if isinstance(self.bank_account_holder_name, Unset):
            bank_account_holder_name = UNSET
        else:
            bank_account_holder_name = self.bank_account_holder_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_number": account_number,
                "bsb_number": bsb_number,
            }
        )
        if bank_account_holder_name is not UNSET:
            field_dict["bank_account_holder_name"] = bank_account_holder_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_number = d.pop("account_number")

        bsb_number = d.pop("bsb_number")

        def _parse_bank_account_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_account_holder_name = _parse_bank_account_holder_name(d.pop("bank_account_holder_name", UNSET))

        becs_bank_debit_additional_data = cls(
            account_number=account_number,
            bsb_number=bsb_number,
            bank_account_holder_name=bank_account_holder_name,
        )

        becs_bank_debit_additional_data.additional_properties = d
        return becs_bank_debit_additional_data

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
