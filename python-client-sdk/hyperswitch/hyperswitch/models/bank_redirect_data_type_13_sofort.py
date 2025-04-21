from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_redirect_billing import BankRedirectBilling


T = TypeVar("T", bound="BankRedirectDataType13Sofort")


@_attrs_define
class BankRedirectDataType13Sofort:
    """
    Attributes:
        country (CountryAlpha2):
        billing_details (Union['BankRedirectBilling', None, Unset]):
        preferred_language (Union[None, Unset, str]): The preferred language Example: en.
    """

    country: CountryAlpha2
    billing_details: Union["BankRedirectBilling", None, Unset] = UNSET
    preferred_language: Union[None, Unset, str] = UNSET
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

        preferred_language: Union[None, Unset, str]
        if isinstance(self.preferred_language, Unset):
            preferred_language = UNSET
        else:
            preferred_language = self.preferred_language

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "country": country,
            }
        )
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details
        if preferred_language is not UNSET:
            field_dict["preferred_language"] = preferred_language

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

        def _parse_preferred_language(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        preferred_language = _parse_preferred_language(d.pop("preferred_language", UNSET))

        bank_redirect_data_type_13_sofort = cls(
            country=country,
            billing_details=billing_details,
            preferred_language=preferred_language,
        )

        bank_redirect_data_type_13_sofort.additional_properties = d
        return bank_redirect_data_type_13_sofort

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
