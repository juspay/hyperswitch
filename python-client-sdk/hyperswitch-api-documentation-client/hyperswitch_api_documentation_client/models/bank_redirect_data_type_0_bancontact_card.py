from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_redirect_billing import BankRedirectBilling


T = TypeVar("T", bound="BankRedirectDataType0BancontactCard")


@_attrs_define
class BankRedirectDataType0BancontactCard:
    """
    Attributes:
        card_number (str): The card number Example: 4242424242424242.
        card_exp_month (str): The card's expiry month Example: 24.
        card_exp_year (str): The card's expiry year Example: 24.
        card_holder_name (str): The card holder's name Example: John Test.
        billing_details (Union['BankRedirectBilling', None, Unset]):
    """

    card_number: str
    card_exp_month: str
    card_exp_year: str
    card_holder_name: str
    billing_details: Union["BankRedirectBilling", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_redirect_billing import BankRedirectBilling

        card_number = self.card_number

        card_exp_month = self.card_exp_month

        card_exp_year = self.card_exp_year

        card_holder_name = self.card_holder_name

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, BankRedirectBilling):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_number": card_number,
                "card_exp_month": card_exp_month,
                "card_exp_year": card_exp_year,
                "card_holder_name": card_holder_name,
            }
        )
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_redirect_billing import BankRedirectBilling

        d = dict(src_dict)
        card_number = d.pop("card_number")

        card_exp_month = d.pop("card_exp_month")

        card_exp_year = d.pop("card_exp_year")

        card_holder_name = d.pop("card_holder_name")

        def _parse_billing_details(data: object) -> Union["BankRedirectBilling", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_details_type_1 = BankRedirectBilling.from_dict(data)

                return billing_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BankRedirectBilling", None, Unset], data)

        billing_details = _parse_billing_details(d.pop("billing_details", UNSET))

        bank_redirect_data_type_0_bancontact_card = cls(
            card_number=card_number,
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            billing_details=billing_details,
        )

        bank_redirect_data_type_0_bancontact_card.additional_properties = d
        return bank_redirect_data_type_0_bancontact_card

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
