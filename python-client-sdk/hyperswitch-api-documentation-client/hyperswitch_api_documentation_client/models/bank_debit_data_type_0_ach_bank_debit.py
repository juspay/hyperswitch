from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_debit_billing import BankDebitBilling


T = TypeVar("T", bound="BankDebitDataType0AchBankDebit")


@_attrs_define
class BankDebitDataType0AchBankDebit:
    """Payment Method data for Ach bank debit

    Attributes:
        account_number (str): Account number for ach bank debit payment Example: 000123456789.
        routing_number (str): Routing number for ach bank debit payment Example: 110000000.
        card_holder_name (str):  Example: John Test.
        bank_account_holder_name (str):  Example: John Doe.
        bank_name (str):  Example: ACH.
        bank_type (str):  Example: Checking.
        bank_holder_type (str):  Example: Personal.
        billing_details (Union['BankDebitBilling', None, Unset]):
    """

    account_number: str
    routing_number: str
    card_holder_name: str
    bank_account_holder_name: str
    bank_name: str
    bank_type: str
    bank_holder_type: str
    billing_details: Union["BankDebitBilling", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_debit_billing import BankDebitBilling

        account_number = self.account_number

        routing_number = self.routing_number

        card_holder_name = self.card_holder_name

        bank_account_holder_name = self.bank_account_holder_name

        bank_name = self.bank_name

        bank_type = self.bank_type

        bank_holder_type = self.bank_holder_type

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, BankDebitBilling):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_number": account_number,
                "routing_number": routing_number,
                "card_holder_name": card_holder_name,
                "bank_account_holder_name": bank_account_holder_name,
                "bank_name": bank_name,
                "bank_type": bank_type,
                "bank_holder_type": bank_holder_type,
            }
        )
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_debit_billing import BankDebitBilling

        d = dict(src_dict)
        account_number = d.pop("account_number")

        routing_number = d.pop("routing_number")

        card_holder_name = d.pop("card_holder_name")

        bank_account_holder_name = d.pop("bank_account_holder_name")

        bank_name = d.pop("bank_name")

        bank_type = d.pop("bank_type")

        bank_holder_type = d.pop("bank_holder_type")

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

        bank_debit_data_type_0_ach_bank_debit = cls(
            account_number=account_number,
            routing_number=routing_number,
            card_holder_name=card_holder_name,
            bank_account_holder_name=bank_account_holder_name,
            bank_name=bank_name,
            bank_type=bank_type,
            bank_holder_type=bank_holder_type,
            billing_details=billing_details,
        )

        bank_debit_data_type_0_ach_bank_debit.additional_properties = d
        return bank_debit_data_type_0_ach_bank_debit

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
