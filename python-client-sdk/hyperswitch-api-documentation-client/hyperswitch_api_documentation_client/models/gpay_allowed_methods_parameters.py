from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.gpay_billing_address_parameters import GpayBillingAddressParameters


T = TypeVar("T", bound="GpayAllowedMethodsParameters")


@_attrs_define
class GpayAllowedMethodsParameters:
    """
    Attributes:
        allowed_auth_methods (list[str]): The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
        allowed_card_networks (list[str]): The list of allowed card networks (ex: AMEX,JCB etc)
        billing_address_required (Union[None, Unset, bool]): Is billing address required
        billing_address_parameters (Union['GpayBillingAddressParameters', None, Unset]):
        assurance_details_required (Union[None, Unset, bool]): Whether assurance details are required
    """

    allowed_auth_methods: list[str]
    allowed_card_networks: list[str]
    billing_address_required: Union[None, Unset, bool] = UNSET
    billing_address_parameters: Union["GpayBillingAddressParameters", None, Unset] = UNSET
    assurance_details_required: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.gpay_billing_address_parameters import GpayBillingAddressParameters

        allowed_auth_methods = self.allowed_auth_methods

        allowed_card_networks = self.allowed_card_networks

        billing_address_required: Union[None, Unset, bool]
        if isinstance(self.billing_address_required, Unset):
            billing_address_required = UNSET
        else:
            billing_address_required = self.billing_address_required

        billing_address_parameters: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_address_parameters, Unset):
            billing_address_parameters = UNSET
        elif isinstance(self.billing_address_parameters, GpayBillingAddressParameters):
            billing_address_parameters = self.billing_address_parameters.to_dict()
        else:
            billing_address_parameters = self.billing_address_parameters

        assurance_details_required: Union[None, Unset, bool]
        if isinstance(self.assurance_details_required, Unset):
            assurance_details_required = UNSET
        else:
            assurance_details_required = self.assurance_details_required

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "allowed_auth_methods": allowed_auth_methods,
                "allowed_card_networks": allowed_card_networks,
            }
        )
        if billing_address_required is not UNSET:
            field_dict["billing_address_required"] = billing_address_required
        if billing_address_parameters is not UNSET:
            field_dict["billing_address_parameters"] = billing_address_parameters
        if assurance_details_required is not UNSET:
            field_dict["assurance_details_required"] = assurance_details_required

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.gpay_billing_address_parameters import GpayBillingAddressParameters

        d = dict(src_dict)
        allowed_auth_methods = cast(list[str], d.pop("allowed_auth_methods"))

        allowed_card_networks = cast(list[str], d.pop("allowed_card_networks"))

        def _parse_billing_address_required(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        billing_address_required = _parse_billing_address_required(d.pop("billing_address_required", UNSET))

        def _parse_billing_address_parameters(data: object) -> Union["GpayBillingAddressParameters", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_address_parameters_type_1 = GpayBillingAddressParameters.from_dict(data)

                return billing_address_parameters_type_1
            except:  # noqa: E722
                pass
            return cast(Union["GpayBillingAddressParameters", None, Unset], data)

        billing_address_parameters = _parse_billing_address_parameters(d.pop("billing_address_parameters", UNSET))

        def _parse_assurance_details_required(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        assurance_details_required = _parse_assurance_details_required(d.pop("assurance_details_required", UNSET))

        gpay_allowed_methods_parameters = cls(
            allowed_auth_methods=allowed_auth_methods,
            allowed_card_networks=allowed_card_networks,
            billing_address_required=billing_address_required,
            billing_address_parameters=billing_address_parameters,
            assurance_details_required=assurance_details_required,
        )

        gpay_allowed_methods_parameters.additional_properties = d
        return gpay_allowed_methods_parameters

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
