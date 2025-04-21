from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PaypalAdditionalData")


@_attrs_define
class PaypalAdditionalData:
    """Masked payout method details for paypal wallet payout method

    Attributes:
        email (Union[None, Unset, str]): Email linked with paypal account Example: john.doe@example.com.
        telephone_number (Union[None, Unset, str]): mobile number linked to paypal account Example: ******* 3349.
        paypal_id (Union[None, Unset, str]): id of the paypal account Example: G83K ***** HCQ2.
    """

    email: Union[None, Unset, str] = UNSET
    telephone_number: Union[None, Unset, str] = UNSET
    paypal_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        email: Union[None, Unset, str]
        if isinstance(self.email, Unset):
            email = UNSET
        else:
            email = self.email

        telephone_number: Union[None, Unset, str]
        if isinstance(self.telephone_number, Unset):
            telephone_number = UNSET
        else:
            telephone_number = self.telephone_number

        paypal_id: Union[None, Unset, str]
        if isinstance(self.paypal_id, Unset):
            paypal_id = UNSET
        else:
            paypal_id = self.paypal_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if email is not UNSET:
            field_dict["email"] = email
        if telephone_number is not UNSET:
            field_dict["telephone_number"] = telephone_number
        if paypal_id is not UNSET:
            field_dict["paypal_id"] = paypal_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        email = _parse_email(d.pop("email", UNSET))

        def _parse_telephone_number(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        telephone_number = _parse_telephone_number(d.pop("telephone_number", UNSET))

        def _parse_paypal_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        paypal_id = _parse_paypal_id(d.pop("paypal_id", UNSET))

        paypal_additional_data = cls(
            email=email,
            telephone_number=telephone_number,
            paypal_id=paypal_id,
        )

        paypal_additional_data.additional_properties = d
        return paypal_additional_data

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
