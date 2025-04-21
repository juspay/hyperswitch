from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.gpay_allowed_methods_parameters import GpayAllowedMethodsParameters
    from ..models.gpay_tokenization_specification import GpayTokenizationSpecification


T = TypeVar("T", bound="GpayAllowedPaymentMethods")


@_attrs_define
class GpayAllowedPaymentMethods:
    """
    Attributes:
        type_ (str): The type of payment method
        parameters (GpayAllowedMethodsParameters):
        tokenization_specification (GpayTokenizationSpecification):
    """

    type_: str
    parameters: "GpayAllowedMethodsParameters"
    tokenization_specification: "GpayTokenizationSpecification"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_

        parameters = self.parameters.to_dict()

        tokenization_specification = self.tokenization_specification.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "parameters": parameters,
                "tokenization_specification": tokenization_specification,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.gpay_allowed_methods_parameters import GpayAllowedMethodsParameters
        from ..models.gpay_tokenization_specification import GpayTokenizationSpecification

        d = dict(src_dict)
        type_ = d.pop("type")

        parameters = GpayAllowedMethodsParameters.from_dict(d.pop("parameters"))

        tokenization_specification = GpayTokenizationSpecification.from_dict(d.pop("tokenization_specification"))

        gpay_allowed_payment_methods = cls(
            type_=type_,
            parameters=parameters,
            tokenization_specification=tokenization_specification,
        )

        gpay_allowed_payment_methods.additional_properties = d
        return gpay_allowed_payment_methods

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
