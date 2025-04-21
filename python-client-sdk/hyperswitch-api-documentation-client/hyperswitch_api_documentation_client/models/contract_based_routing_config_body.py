from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.contract_based_time_scale import ContractBasedTimeScale
from ..types import UNSET, Unset

T = TypeVar("T", bound="ContractBasedRoutingConfigBody")


@_attrs_define
class ContractBasedRoutingConfigBody:
    """
    Attributes:
        constants (Union[None, Unset, list[float]]):
        time_scale (Union[ContractBasedTimeScale, None, Unset]):
    """

    constants: Union[None, Unset, list[float]] = UNSET
    time_scale: Union[ContractBasedTimeScale, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        constants: Union[None, Unset, list[float]]
        if isinstance(self.constants, Unset):
            constants = UNSET
        elif isinstance(self.constants, list):
            constants = self.constants

        else:
            constants = self.constants

        time_scale: Union[None, Unset, str]
        if isinstance(self.time_scale, Unset):
            time_scale = UNSET
        elif isinstance(self.time_scale, ContractBasedTimeScale):
            time_scale = self.time_scale.value
        else:
            time_scale = self.time_scale

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if constants is not UNSET:
            field_dict["constants"] = constants
        if time_scale is not UNSET:
            field_dict["time_scale"] = time_scale

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_constants(data: object) -> Union[None, Unset, list[float]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                constants_type_0 = cast(list[float], data)

                return constants_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[float]], data)

        constants = _parse_constants(d.pop("constants", UNSET))

        def _parse_time_scale(data: object) -> Union[ContractBasedTimeScale, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                time_scale_type_1 = ContractBasedTimeScale(data)

                return time_scale_type_1
            except:  # noqa: E722
                pass
            return cast(Union[ContractBasedTimeScale, None, Unset], data)

        time_scale = _parse_time_scale(d.pop("time_scale", UNSET))

        contract_based_routing_config_body = cls(
            constants=constants,
            time_scale=time_scale,
        )

        contract_based_routing_config_body.additional_properties = d
        return contract_based_routing_config_body

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
