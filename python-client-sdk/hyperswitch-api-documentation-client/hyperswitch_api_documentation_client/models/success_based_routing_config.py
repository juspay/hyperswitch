from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.dynamic_routing_config_params import DynamicRoutingConfigParams
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.success_based_routing_config_body import SuccessBasedRoutingConfigBody


T = TypeVar("T", bound="SuccessBasedRoutingConfig")


@_attrs_define
class SuccessBasedRoutingConfig:
    """
    Attributes:
        params (Union[None, Unset, list[DynamicRoutingConfigParams]]):
        config (Union['SuccessBasedRoutingConfigBody', None, Unset]):
    """

    params: Union[None, Unset, list[DynamicRoutingConfigParams]] = UNSET
    config: Union["SuccessBasedRoutingConfigBody", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.success_based_routing_config_body import SuccessBasedRoutingConfigBody

        params: Union[None, Unset, list[str]]
        if isinstance(self.params, Unset):
            params = UNSET
        elif isinstance(self.params, list):
            params = []
            for params_type_0_item_data in self.params:
                params_type_0_item = params_type_0_item_data.value
                params.append(params_type_0_item)

        else:
            params = self.params

        config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.config, Unset):
            config = UNSET
        elif isinstance(self.config, SuccessBasedRoutingConfigBody):
            config = self.config.to_dict()
        else:
            config = self.config

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if params is not UNSET:
            field_dict["params"] = params
        if config is not UNSET:
            field_dict["config"] = config

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.success_based_routing_config_body import SuccessBasedRoutingConfigBody

        d = dict(src_dict)

        def _parse_params(data: object) -> Union[None, Unset, list[DynamicRoutingConfigParams]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                params_type_0 = []
                _params_type_0 = data
                for params_type_0_item_data in _params_type_0:
                    params_type_0_item = DynamicRoutingConfigParams(params_type_0_item_data)

                    params_type_0.append(params_type_0_item)

                return params_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[DynamicRoutingConfigParams]], data)

        params = _parse_params(d.pop("params", UNSET))

        def _parse_config(data: object) -> Union["SuccessBasedRoutingConfigBody", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                config_type_1 = SuccessBasedRoutingConfigBody.from_dict(data)

                return config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SuccessBasedRoutingConfigBody", None, Unset], data)

        config = _parse_config(d.pop("config", UNSET))

        success_based_routing_config = cls(
            params=params,
            config=config,
        )

        success_based_routing_config.additional_properties = d
        return success_based_routing_config

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
