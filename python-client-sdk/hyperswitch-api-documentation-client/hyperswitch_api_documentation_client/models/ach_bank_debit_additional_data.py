from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.bank_holder_type import BankHolderType
from ..models.bank_names import BankNames
from ..models.bank_type import BankType
from ..types import UNSET, Unset

T = TypeVar("T", bound="AchBankDebitAdditionalData")


@_attrs_define
class AchBankDebitAdditionalData:
    """
    Attributes:
        account_number (str): Partially masked account number for ach bank debit payment Example: 0001****3456.
        routing_number (str): Partially masked routing number for ach bank debit payment Example: 110***000.
        card_holder_name (Union[None, Unset, str]): Card holder's name Example: John Doe.
        bank_account_holder_name (Union[None, Unset, str]): Bank account's owner name Example: John Doe.
        bank_name (Union[BankNames, None, Unset]):
        bank_type (Union[BankType, None, Unset]):
        bank_holder_type (Union[BankHolderType, None, Unset]):
    """

    account_number: str
    routing_number: str
    card_holder_name: Union[None, Unset, str] = UNSET
    bank_account_holder_name: Union[None, Unset, str] = UNSET
    bank_name: Union[BankNames, None, Unset] = UNSET
    bank_type: Union[BankType, None, Unset] = UNSET
    bank_holder_type: Union[BankHolderType, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_number = self.account_number

        routing_number = self.routing_number

        card_holder_name: Union[None, Unset, str]
        if isinstance(self.card_holder_name, Unset):
            card_holder_name = UNSET
        else:
            card_holder_name = self.card_holder_name

        bank_account_holder_name: Union[None, Unset, str]
        if isinstance(self.bank_account_holder_name, Unset):
            bank_account_holder_name = UNSET
        else:
            bank_account_holder_name = self.bank_account_holder_name

        bank_name: Union[None, Unset, str]
        if isinstance(self.bank_name, Unset):
            bank_name = UNSET
        elif isinstance(self.bank_name, BankNames):
            bank_name = self.bank_name.value
        else:
            bank_name = self.bank_name

        bank_type: Union[None, Unset, str]
        if isinstance(self.bank_type, Unset):
            bank_type = UNSET
        elif isinstance(self.bank_type, BankType):
            bank_type = self.bank_type.value
        else:
            bank_type = self.bank_type

        bank_holder_type: Union[None, Unset, str]
        if isinstance(self.bank_holder_type, Unset):
            bank_holder_type = UNSET
        elif isinstance(self.bank_holder_type, BankHolderType):
            bank_holder_type = self.bank_holder_type.value
        else:
            bank_holder_type = self.bank_holder_type

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_number": account_number,
                "routing_number": routing_number,
            }
        )
        if card_holder_name is not UNSET:
            field_dict["card_holder_name"] = card_holder_name
        if bank_account_holder_name is not UNSET:
            field_dict["bank_account_holder_name"] = bank_account_holder_name
        if bank_name is not UNSET:
            field_dict["bank_name"] = bank_name
        if bank_type is not UNSET:
            field_dict["bank_type"] = bank_type
        if bank_holder_type is not UNSET:
            field_dict["bank_holder_type"] = bank_holder_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_number = d.pop("account_number")

        routing_number = d.pop("routing_number")

        def _parse_card_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_holder_name = _parse_card_holder_name(d.pop("card_holder_name", UNSET))

        def _parse_bank_account_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_account_holder_name = _parse_bank_account_holder_name(d.pop("bank_account_holder_name", UNSET))

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

        def _parse_bank_type(data: object) -> Union[BankType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                bank_type_type_1 = BankType(data)

                return bank_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[BankType, None, Unset], data)

        bank_type = _parse_bank_type(d.pop("bank_type", UNSET))

        def _parse_bank_holder_type(data: object) -> Union[BankHolderType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                bank_holder_type_type_1 = BankHolderType(data)

                return bank_holder_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[BankHolderType, None, Unset], data)

        bank_holder_type = _parse_bank_holder_type(d.pop("bank_holder_type", UNSET))

        ach_bank_debit_additional_data = cls(
            account_number=account_number,
            routing_number=routing_number,
            card_holder_name=card_holder_name,
            bank_account_holder_name=bank_account_holder_name,
            bank_name=bank_name,
            bank_type=bank_type,
            bank_holder_type=bank_holder_type,
        )

        ach_bank_debit_additional_data.additional_properties = d
        return ach_bank_debit_additional_data

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
