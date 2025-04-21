from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define

from ..models.country_alpha_2 import CountryAlpha2

T = TypeVar("T", bound="PrimaryBusinessDetails")


@_attrs_define
class PrimaryBusinessDetails:
    """
    Attributes:
        country (CountryAlpha2):
        business (str):  Example: food.
    """

    country: CountryAlpha2
    business: str

    def to_dict(self) -> dict[str, Any]:
        country = self.country.value

        business = self.business

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "country": country,
                "business": business,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        country = CountryAlpha2(d.pop("country"))

        business = d.pop("business")

        primary_business_details = cls(
            country=country,
            business=business,
        )

        return primary_business_details
