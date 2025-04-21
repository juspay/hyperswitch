from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.contract_based_routing_config_body import ContractBasedRoutingConfigBody
    from ..models.label_information import LabelInformation


T = TypeVar("T", bound="ContractBasedRoutingConfig")


@_attrs_define
class ContractBasedRoutingConfig:
    """
    Attributes:
        config (Union['ContractBasedRoutingConfigBody', None, Unset]):
        label_info (Union[None, Unset, list['LabelInformation']]):
    """

    config: Union["ContractBasedRoutingConfigBody", None, Unset] = UNSET
    label_info: Union[None, Unset, list["LabelInformation"]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.contract_based_routing_config_body import ContractBasedRoutingConfigBody

        config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.config, Unset):
            config = UNSET
        elif isinstance(self.config, ContractBasedRoutingConfigBody):
            config = self.config.to_dict()
        else:
            config = self.config

        label_info: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.label_info, Unset):
            label_info = UNSET
        elif isinstance(self.label_info, list):
            label_info = []
            for label_info_type_0_item_data in self.label_info:
                label_info_type_0_item = label_info_type_0_item_data.to_dict()
                label_info.append(label_info_type_0_item)

        else:
            label_info = self.label_info

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if config is not UNSET:
            field_dict["config"] = config
        if label_info is not UNSET:
            field_dict["label_info"] = label_info

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.contract_based_routing_config_body import ContractBasedRoutingConfigBody
        from ..models.label_information import LabelInformation

        d = dict(src_dict)

        def _parse_config(data: object) -> Union["ContractBasedRoutingConfigBody", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                config_type_1 = ContractBasedRoutingConfigBody.from_dict(data)

                return config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ContractBasedRoutingConfigBody", None, Unset], data)

        config = _parse_config(d.pop("config", UNSET))

        def _parse_label_info(data: object) -> Union[None, Unset, list["LabelInformation"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                label_info_type_0 = []
                _label_info_type_0 = data
                for label_info_type_0_item_data in _label_info_type_0:
                    label_info_type_0_item = LabelInformation.from_dict(label_info_type_0_item_data)

                    label_info_type_0.append(label_info_type_0_item)

                return label_info_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["LabelInformation"]], data)

        label_info = _parse_label_info(d.pop("label_info", UNSET))

        contract_based_routing_config = cls(
            config=config,
            label_info=label_info,
        )

        contract_based_routing_config.additional_properties = d
        return contract_based_routing_config

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
