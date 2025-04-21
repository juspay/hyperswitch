from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address import Address


T = TypeVar("T", bound="PaymentMethodDataResponseWithBilling")


@_attrs_define
class PaymentMethodDataResponseWithBilling:
    """
    Attributes:
        billing (Union['Address', None, Unset]):
    """

    billing: Union["Address", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.address import Address

        billing: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing, Unset):
            billing = UNSET
        elif isinstance(self.billing, Address):
            billing = self.billing.to_dict()
        else:
            billing = self.billing

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if billing is not UNSET:
            field_dict["billing"] = billing

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address import Address

        d = dict(src_dict)

        def _parse_billing(data: object) -> Union["Address", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_type_1 = Address.from_dict(data)

                return billing_type_1
            except:  # noqa: E722
                pass
            return cast(Union["Address", None, Unset], data)

        billing = _parse_billing(d.pop("billing", UNSET))

        payment_method_data_response_with_billing = cls(
            billing=billing,
        )

        payment_method_data_response_with_billing.additional_properties = d
        return payment_method_data_response_with_billing

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
