from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.three_ds_completion_indicator import ThreeDsCompletionIndicator
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address import Address


T = TypeVar("T", bound="PaymentsCompleteAuthorizeRequest")


@_attrs_define
class PaymentsCompleteAuthorizeRequest:
    """
    Attributes:
        client_secret (str): Client Secret
        shipping (Union['Address', None, Unset]):
        threeds_method_comp_ind (Union[None, ThreeDsCompletionIndicator, Unset]):
    """

    client_secret: str
    shipping: Union["Address", None, Unset] = UNSET
    threeds_method_comp_ind: Union[None, ThreeDsCompletionIndicator, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.address import Address

        client_secret = self.client_secret

        shipping: Union[None, Unset, dict[str, Any]]
        if isinstance(self.shipping, Unset):
            shipping = UNSET
        elif isinstance(self.shipping, Address):
            shipping = self.shipping.to_dict()
        else:
            shipping = self.shipping

        threeds_method_comp_ind: Union[None, Unset, str]
        if isinstance(self.threeds_method_comp_ind, Unset):
            threeds_method_comp_ind = UNSET
        elif isinstance(self.threeds_method_comp_ind, ThreeDsCompletionIndicator):
            threeds_method_comp_ind = self.threeds_method_comp_ind.value
        else:
            threeds_method_comp_ind = self.threeds_method_comp_ind

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "client_secret": client_secret,
            }
        )
        if shipping is not UNSET:
            field_dict["shipping"] = shipping
        if threeds_method_comp_ind is not UNSET:
            field_dict["threeds_method_comp_ind"] = threeds_method_comp_ind

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address import Address

        d = dict(src_dict)
        client_secret = d.pop("client_secret")

        def _parse_shipping(data: object) -> Union["Address", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                shipping_type_1 = Address.from_dict(data)

                return shipping_type_1
            except:  # noqa: E722
                pass
            return cast(Union["Address", None, Unset], data)

        shipping = _parse_shipping(d.pop("shipping", UNSET))

        def _parse_threeds_method_comp_ind(data: object) -> Union[None, ThreeDsCompletionIndicator, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                threeds_method_comp_ind_type_1 = ThreeDsCompletionIndicator(data)

                return threeds_method_comp_ind_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, ThreeDsCompletionIndicator, Unset], data)

        threeds_method_comp_ind = _parse_threeds_method_comp_ind(d.pop("threeds_method_comp_ind", UNSET))

        payments_complete_authorize_request = cls(
            client_secret=client_secret,
            shipping=shipping,
            threeds_method_comp_ind=threeds_method_comp_ind,
        )

        payments_complete_authorize_request.additional_properties = d
        return payments_complete_authorize_request

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
