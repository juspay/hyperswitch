from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_routing_algorithm import MerchantRoutingAlgorithm


T = TypeVar("T", bound="RoutingRetrieveResponse")


@_attrs_define
class RoutingRetrieveResponse:
    """Response of the retrieved routing configs for a merchant account

    Attributes:
        algorithm (Union['MerchantRoutingAlgorithm', None, Unset]):
    """

    algorithm: Union["MerchantRoutingAlgorithm", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_routing_algorithm import MerchantRoutingAlgorithm

        algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.algorithm, Unset):
            algorithm = UNSET
        elif isinstance(self.algorithm, MerchantRoutingAlgorithm):
            algorithm = self.algorithm.to_dict()
        else:
            algorithm = self.algorithm

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if algorithm is not UNSET:
            field_dict["algorithm"] = algorithm

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_routing_algorithm import MerchantRoutingAlgorithm

        d = dict(src_dict)

        def _parse_algorithm(data: object) -> Union["MerchantRoutingAlgorithm", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                algorithm_type_1 = MerchantRoutingAlgorithm.from_dict(data)

                return algorithm_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantRoutingAlgorithm", None, Unset], data)

        algorithm = _parse_algorithm(d.pop("algorithm", UNSET))

        routing_retrieve_response = cls(
            algorithm=algorithm,
        )

        routing_retrieve_response.additional_properties = d
        return routing_retrieve_response

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
