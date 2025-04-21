from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.klarna_sdk_payment_method_response import KlarnaSdkPaymentMethodResponse


T = TypeVar("T", bound="PaylaterResponse")


@_attrs_define
class PaylaterResponse:
    """
    Attributes:
        klarna_sdk (Union['KlarnaSdkPaymentMethodResponse', None, Unset]):
    """

    klarna_sdk: Union["KlarnaSdkPaymentMethodResponse", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.klarna_sdk_payment_method_response import KlarnaSdkPaymentMethodResponse

        klarna_sdk: Union[None, Unset, dict[str, Any]]
        if isinstance(self.klarna_sdk, Unset):
            klarna_sdk = UNSET
        elif isinstance(self.klarna_sdk, KlarnaSdkPaymentMethodResponse):
            klarna_sdk = self.klarna_sdk.to_dict()
        else:
            klarna_sdk = self.klarna_sdk

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if klarna_sdk is not UNSET:
            field_dict["klarna_sdk"] = klarna_sdk

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.klarna_sdk_payment_method_response import KlarnaSdkPaymentMethodResponse

        d = dict(src_dict)

        def _parse_klarna_sdk(data: object) -> Union["KlarnaSdkPaymentMethodResponse", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                klarna_sdk_type_1 = KlarnaSdkPaymentMethodResponse.from_dict(data)

                return klarna_sdk_type_1
            except:  # noqa: E722
                pass
            return cast(Union["KlarnaSdkPaymentMethodResponse", None, Unset], data)

        klarna_sdk = _parse_klarna_sdk(d.pop("klarna_sdk", UNSET))

        paylater_response = cls(
            klarna_sdk=klarna_sdk,
        )

        paylater_response.additional_properties = d
        return paylater_response

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
