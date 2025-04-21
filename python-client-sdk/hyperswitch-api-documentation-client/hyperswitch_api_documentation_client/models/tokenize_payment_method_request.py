from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TokenizePaymentMethodRequest")


@_attrs_define
class TokenizePaymentMethodRequest:
    """
    Attributes:
        card_cvc (Union[None, Unset, str]): The CVC number for the card Example: 242.
    """

    card_cvc: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_cvc: Union[None, Unset, str]
        if isinstance(self.card_cvc, Unset):
            card_cvc = UNSET
        else:
            card_cvc = self.card_cvc

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if card_cvc is not UNSET:
            field_dict["card_cvc"] = card_cvc

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_card_cvc(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_cvc = _parse_card_cvc(d.pop("card_cvc", UNSET))

        tokenize_payment_method_request = cls(
            card_cvc=card_cvc,
        )

        tokenize_payment_method_request.additional_properties = d
        return tokenize_payment_method_request

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
