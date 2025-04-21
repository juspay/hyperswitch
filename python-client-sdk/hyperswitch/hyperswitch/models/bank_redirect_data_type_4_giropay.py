from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_redirect_billing import BankRedirectBilling


T = TypeVar("T", bound="BankRedirectDataType4Giropay")


@_attrs_define
class BankRedirectDataType4Giropay:
    """
    Attributes:
        country (CountryAlpha2):
        billing_details (Union['BankRedirectBilling', None, Unset]):
        bank_account_bic (Union[None, Unset, str]): Bank account bic code
        bank_account_iban (Union[None, Unset, str]): Bank account iban
    """

    country: CountryAlpha2
    billing_details: Union["BankRedirectBilling", None, Unset] = UNSET
    bank_account_bic: Union[None, Unset, str] = UNSET
    bank_account_iban: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_redirect_billing import BankRedirectBilling

        country = self.country.value

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, BankRedirectBilling):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        bank_account_bic: Union[None, Unset, str]
        if isinstance(self.bank_account_bic, Unset):
            bank_account_bic = UNSET
        else:
            bank_account_bic = self.bank_account_bic

        bank_account_iban: Union[None, Unset, str]
        if isinstance(self.bank_account_iban, Unset):
            bank_account_iban = UNSET
        else:
            bank_account_iban = self.bank_account_iban

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "country": country,
            }
        )
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details
        if bank_account_bic is not UNSET:
            field_dict["bank_account_bic"] = bank_account_bic
        if bank_account_iban is not UNSET:
            field_dict["bank_account_iban"] = bank_account_iban

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_redirect_billing import BankRedirectBilling

        d = dict(src_dict)
        country = CountryAlpha2(d.pop("country"))

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

        def _parse_bank_account_bic(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_account_bic = _parse_bank_account_bic(d.pop("bank_account_bic", UNSET))

        def _parse_bank_account_iban(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_account_iban = _parse_bank_account_iban(d.pop("bank_account_iban", UNSET))

        bank_redirect_data_type_4_giropay = cls(
            country=country,
            billing_details=billing_details,
            bank_account_bic=bank_account_bic,
            bank_account_iban=bank_account_iban,
        )

        bank_redirect_data_type_4_giropay.additional_properties = d
        return bank_redirect_data_type_4_giropay

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
