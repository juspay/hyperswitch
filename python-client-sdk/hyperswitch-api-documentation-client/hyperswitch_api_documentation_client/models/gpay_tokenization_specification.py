from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.gpay_token_parameters import GpayTokenParameters


T = TypeVar("T", bound="GpayTokenizationSpecification")


@_attrs_define
class GpayTokenizationSpecification:
    """
    Attributes:
        type_ (str): The token specification type(ex: PAYMENT_GATEWAY)
        parameters (GpayTokenParameters):
    """

    type_: str
    parameters: "GpayTokenParameters"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_

        parameters = self.parameters.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "parameters": parameters,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.gpay_token_parameters import GpayTokenParameters

        d = dict(src_dict)
        type_ = d.pop("type")

        parameters = GpayTokenParameters.from_dict(d.pop("parameters"))

        gpay_tokenization_specification = cls(
            type_=type_,
            parameters=parameters,
        )

        gpay_tokenization_specification.additional_properties = d
        return gpay_tokenization_specification

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
