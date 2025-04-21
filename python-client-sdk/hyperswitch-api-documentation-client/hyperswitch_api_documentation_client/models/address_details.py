from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="AddressDetails")


@_attrs_define
class AddressDetails:
    """Address details

    Attributes:
        city (Union[None, Unset, str]): The address city Example: New York.
        country (Union[CountryAlpha2, None, Unset]):
        line1 (Union[None, Unset, str]): The first line of the address Example: 123, King Street.
        line2 (Union[None, Unset, str]): The second line of the address Example: Powelson Avenue.
        line3 (Union[None, Unset, str]): The third line of the address Example: Bridgewater.
        zip_ (Union[None, Unset, str]): The zip/postal code for the address Example: 08807.
        state (Union[None, Unset, str]): The address state Example: New York.
        first_name (Union[None, Unset, str]): The first name for the address Example: John.
        last_name (Union[None, Unset, str]): The last name for the address Example: Doe.
    """

    city: Union[None, Unset, str] = UNSET
    country: Union[CountryAlpha2, None, Unset] = UNSET
    line1: Union[None, Unset, str] = UNSET
    line2: Union[None, Unset, str] = UNSET
    line3: Union[None, Unset, str] = UNSET
    zip_: Union[None, Unset, str] = UNSET
    state: Union[None, Unset, str] = UNSET
    first_name: Union[None, Unset, str] = UNSET
    last_name: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        city: Union[None, Unset, str]
        if isinstance(self.city, Unset):
            city = UNSET
        else:
            city = self.city

        country: Union[None, Unset, str]
        if isinstance(self.country, Unset):
            country = UNSET
        elif isinstance(self.country, CountryAlpha2):
            country = self.country.value
        else:
            country = self.country

        line1: Union[None, Unset, str]
        if isinstance(self.line1, Unset):
            line1 = UNSET
        else:
            line1 = self.line1

        line2: Union[None, Unset, str]
        if isinstance(self.line2, Unset):
            line2 = UNSET
        else:
            line2 = self.line2

        line3: Union[None, Unset, str]
        if isinstance(self.line3, Unset):
            line3 = UNSET
        else:
            line3 = self.line3

        zip_: Union[None, Unset, str]
        if isinstance(self.zip_, Unset):
            zip_ = UNSET
        else:
            zip_ = self.zip_

        state: Union[None, Unset, str]
        if isinstance(self.state, Unset):
            state = UNSET
        else:
            state = self.state

        first_name: Union[None, Unset, str]
        if isinstance(self.first_name, Unset):
            first_name = UNSET
        else:
            first_name = self.first_name

        last_name: Union[None, Unset, str]
        if isinstance(self.last_name, Unset):
            last_name = UNSET
        else:
            last_name = self.last_name

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if city is not UNSET:
            field_dict["city"] = city
        if country is not UNSET:
            field_dict["country"] = country
        if line1 is not UNSET:
            field_dict["line1"] = line1
        if line2 is not UNSET:
            field_dict["line2"] = line2
        if line3 is not UNSET:
            field_dict["line3"] = line3
        if zip_ is not UNSET:
            field_dict["zip"] = zip_
        if state is not UNSET:
            field_dict["state"] = state
        if first_name is not UNSET:
            field_dict["first_name"] = first_name
        if last_name is not UNSET:
            field_dict["last_name"] = last_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_city(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        city = _parse_city(d.pop("city", UNSET))

        def _parse_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                country_type_1 = CountryAlpha2(data)

                return country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        country = _parse_country(d.pop("country", UNSET))

        def _parse_line1(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        line1 = _parse_line1(d.pop("line1", UNSET))

        def _parse_line2(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        line2 = _parse_line2(d.pop("line2", UNSET))

        def _parse_line3(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        line3 = _parse_line3(d.pop("line3", UNSET))

        def _parse_zip_(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        zip_ = _parse_zip_(d.pop("zip", UNSET))

        def _parse_state(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        state = _parse_state(d.pop("state", UNSET))

        def _parse_first_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        first_name = _parse_first_name(d.pop("first_name", UNSET))

        def _parse_last_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        last_name = _parse_last_name(d.pop("last_name", UNSET))

        address_details = cls(
            city=city,
            country=country,
            line1=line1,
            line2=line2,
            line3=line3,
            zip_=zip_,
            state=state,
            first_name=first_name,
            last_name=last_name,
        )

        return address_details
