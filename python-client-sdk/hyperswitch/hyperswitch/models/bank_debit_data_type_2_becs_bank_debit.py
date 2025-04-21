from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_debit_billing import BankDebitBilling


T = TypeVar("T", bound="BankDebitDataType2BecsBankDebit")


@_attrs_define
class BankDebitDataType2BecsBankDebit:
    """
    Attributes:
        account_number (str): Account number for Becs payment method Example: 000123456.
        bsb_number (str): Bank-State-Branch (bsb) number Example: 000000.
        billing_details (Union['BankDebitBilling', None, Unset]):
        bank_account_holder_name (Union[None, Unset, str]): Owner name for bank debit Example: A. Schneider.
    """

    account_number: str
    bsb_number: str
    billing_details: Union["BankDebitBilling", None, Unset] = UNSET
    bank_account_holder_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_debit_billing import BankDebitBilling

        account_number = self.account_number

        bsb_number = self.bsb_number

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, BankDebitBilling):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

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
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details
        if bank_account_holder_name is not UNSET:
            field_dict["bank_account_holder_name"] = bank_account_holder_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_debit_billing import BankDebitBilling

        d = dict(src_dict)
        account_number = d.pop("account_number")

        bsb_number = d.pop("bsb_number")

        def _parse_billing_details(data: object) -> Union["BankDebitBilling", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_details_type_1 = BankDebitBilling.from_dict(data)

                return billing_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BankDebitBilling", None, Unset], data)

        billing_details = _parse_billing_details(d.pop("billing_details", UNSET))

        def _parse_bank_account_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_account_holder_name = _parse_bank_account_holder_name(d.pop("bank_account_holder_name", UNSET))

        bank_debit_data_type_2_becs_bank_debit = cls(
            account_number=account_number,
            bsb_number=bsb_number,
            billing_details=billing_details,
            bank_account_holder_name=bank_account_holder_name,
        )

        bank_debit_data_type_2_becs_bank_debit.additional_properties = d
        return bank_debit_data_type_2_becs_bank_debit

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
