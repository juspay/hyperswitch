from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address_details import AddressDetails


T = TypeVar("T", bound="MerchantDetails")


@_attrs_define
class MerchantDetails:
    """
    Attributes:
        primary_contact_person (Union[None, Unset, str]): The merchant's primary contact name Example: John Doe.
        primary_phone (Union[None, Unset, str]): The merchant's primary phone number Example: 999999999.
        primary_email (Union[None, Unset, str]): The merchant's primary email address Example: johndoe@test.com.
        secondary_contact_person (Union[None, Unset, str]): The merchant's secondary contact name Example: John Doe2.
        secondary_phone (Union[None, Unset, str]): The merchant's secondary phone number Example: 999999988.
        secondary_email (Union[None, Unset, str]): The merchant's secondary email address Example: johndoe2@test.com.
        website (Union[None, Unset, str]): The business website of the merchant Example: www.example.com.
        about_business (Union[None, Unset, str]): A brief description about merchant's business Example: Online Retail
            with a wide selection of organic products for North America.
        address (Union['AddressDetails', None, Unset]):
    """

    primary_contact_person: Union[None, Unset, str] = UNSET
    primary_phone: Union[None, Unset, str] = UNSET
    primary_email: Union[None, Unset, str] = UNSET
    secondary_contact_person: Union[None, Unset, str] = UNSET
    secondary_phone: Union[None, Unset, str] = UNSET
    secondary_email: Union[None, Unset, str] = UNSET
    website: Union[None, Unset, str] = UNSET
    about_business: Union[None, Unset, str] = UNSET
    address: Union["AddressDetails", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.address_details import AddressDetails

        primary_contact_person: Union[None, Unset, str]
        if isinstance(self.primary_contact_person, Unset):
            primary_contact_person = UNSET
        else:
            primary_contact_person = self.primary_contact_person

        primary_phone: Union[None, Unset, str]
        if isinstance(self.primary_phone, Unset):
            primary_phone = UNSET
        else:
            primary_phone = self.primary_phone

        primary_email: Union[None, Unset, str]
        if isinstance(self.primary_email, Unset):
            primary_email = UNSET
        else:
            primary_email = self.primary_email

        secondary_contact_person: Union[None, Unset, str]
        if isinstance(self.secondary_contact_person, Unset):
            secondary_contact_person = UNSET
        else:
            secondary_contact_person = self.secondary_contact_person

        secondary_phone: Union[None, Unset, str]
        if isinstance(self.secondary_phone, Unset):
            secondary_phone = UNSET
        else:
            secondary_phone = self.secondary_phone

        secondary_email: Union[None, Unset, str]
        if isinstance(self.secondary_email, Unset):
            secondary_email = UNSET
        else:
            secondary_email = self.secondary_email

        website: Union[None, Unset, str]
        if isinstance(self.website, Unset):
            website = UNSET
        else:
            website = self.website

        about_business: Union[None, Unset, str]
        if isinstance(self.about_business, Unset):
            about_business = UNSET
        else:
            about_business = self.about_business

        address: Union[None, Unset, dict[str, Any]]
        if isinstance(self.address, Unset):
            address = UNSET
        elif isinstance(self.address, AddressDetails):
            address = self.address.to_dict()
        else:
            address = self.address

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if primary_contact_person is not UNSET:
            field_dict["primary_contact_person"] = primary_contact_person
        if primary_phone is not UNSET:
            field_dict["primary_phone"] = primary_phone
        if primary_email is not UNSET:
            field_dict["primary_email"] = primary_email
        if secondary_contact_person is not UNSET:
            field_dict["secondary_contact_person"] = secondary_contact_person
        if secondary_phone is not UNSET:
            field_dict["secondary_phone"] = secondary_phone
        if secondary_email is not UNSET:
            field_dict["secondary_email"] = secondary_email
        if website is not UNSET:
            field_dict["website"] = website
        if about_business is not UNSET:
            field_dict["about_business"] = about_business
        if address is not UNSET:
            field_dict["address"] = address

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address_details import AddressDetails

        d = dict(src_dict)

        def _parse_primary_contact_person(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        primary_contact_person = _parse_primary_contact_person(d.pop("primary_contact_person", UNSET))

        def _parse_primary_phone(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        primary_phone = _parse_primary_phone(d.pop("primary_phone", UNSET))

        def _parse_primary_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        primary_email = _parse_primary_email(d.pop("primary_email", UNSET))

        def _parse_secondary_contact_person(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        secondary_contact_person = _parse_secondary_contact_person(d.pop("secondary_contact_person", UNSET))

        def _parse_secondary_phone(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        secondary_phone = _parse_secondary_phone(d.pop("secondary_phone", UNSET))

        def _parse_secondary_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        secondary_email = _parse_secondary_email(d.pop("secondary_email", UNSET))

        def _parse_website(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        website = _parse_website(d.pop("website", UNSET))

        def _parse_about_business(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        about_business = _parse_about_business(d.pop("about_business", UNSET))

        def _parse_address(data: object) -> Union["AddressDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                address_type_1 = AddressDetails.from_dict(data)

                return address_type_1
            except:  # noqa: E722
                pass
            return cast(Union["AddressDetails", None, Unset], data)

        address = _parse_address(d.pop("address", UNSET))

        merchant_details = cls(
            primary_contact_person=primary_contact_person,
            primary_phone=primary_phone,
            primary_email=primary_email,
            secondary_contact_person=secondary_contact_person,
            secondary_phone=secondary_phone,
            secondary_email=secondary_email,
            website=website,
            about_business=about_business,
            address=address,
        )

        return merchant_details
