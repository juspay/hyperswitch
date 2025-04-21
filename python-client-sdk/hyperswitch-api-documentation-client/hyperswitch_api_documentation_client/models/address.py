from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address_details import AddressDetails
    from ..models.phone_details import PhoneDetails


T = TypeVar("T", bound="Address")


@_attrs_define
class Address:
    """
    Attributes:
        address (Union['AddressDetails', None, Unset]):
        phone (Union['PhoneDetails', None, Unset]):
        email (Union[None, Unset, str]):
    """

    address: Union["AddressDetails", None, Unset] = UNSET
    phone: Union["PhoneDetails", None, Unset] = UNSET
    email: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.address_details import AddressDetails
        from ..models.phone_details import PhoneDetails

        address: Union[None, Unset, dict[str, Any]]
        if isinstance(self.address, Unset):
            address = UNSET
        elif isinstance(self.address, AddressDetails):
            address = self.address.to_dict()
        else:
            address = self.address

        phone: Union[None, Unset, dict[str, Any]]
        if isinstance(self.phone, Unset):
            phone = UNSET
        elif isinstance(self.phone, PhoneDetails):
            phone = self.phone.to_dict()
        else:
            phone = self.phone

        email: Union[None, Unset, str]
        if isinstance(self.email, Unset):
            email = UNSET
        else:
            email = self.email

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if address is not UNSET:
            field_dict["address"] = address
        if phone is not UNSET:
            field_dict["phone"] = phone
        if email is not UNSET:
            field_dict["email"] = email

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address_details import AddressDetails
        from ..models.phone_details import PhoneDetails

        d = dict(src_dict)

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

        def _parse_phone(data: object) -> Union["PhoneDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                phone_type_1 = PhoneDetails.from_dict(data)

                return phone_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PhoneDetails", None, Unset], data)

        phone = _parse_phone(d.pop("phone", UNSET))

        def _parse_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        email = _parse_email(d.pop("email", UNSET))

        address = cls(
            address=address,
            phone=phone,
            email=email,
        )

        return address
