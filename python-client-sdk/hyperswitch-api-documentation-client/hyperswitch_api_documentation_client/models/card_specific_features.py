from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..models.feature_status import FeatureStatus

T = TypeVar("T", bound="CardSpecificFeatures")


@_attrs_define
class CardSpecificFeatures:
    """
    Attributes:
        three_ds (FeatureStatus): The status of the feature
        no_three_ds (FeatureStatus): The status of the feature
        supported_card_networks (list[CardNetwork]): List of supported card networks
    """

    three_ds: FeatureStatus
    no_three_ds: FeatureStatus
    supported_card_networks: list[CardNetwork]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        three_ds = self.three_ds.value

        no_three_ds = self.no_three_ds.value

        supported_card_networks = []
        for supported_card_networks_item_data in self.supported_card_networks:
            supported_card_networks_item = supported_card_networks_item_data.value
            supported_card_networks.append(supported_card_networks_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "three_ds": three_ds,
                "no_three_ds": no_three_ds,
                "supported_card_networks": supported_card_networks,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        three_ds = FeatureStatus(d.pop("three_ds"))

        no_three_ds = FeatureStatus(d.pop("no_three_ds"))

        supported_card_networks = []
        _supported_card_networks = d.pop("supported_card_networks")
        for supported_card_networks_item_data in _supported_card_networks:
            supported_card_networks_item = CardNetwork(supported_card_networks_item_data)

            supported_card_networks.append(supported_card_networks_item)

        card_specific_features = cls(
            three_ds=three_ds,
            no_three_ds=no_three_ds,
            supported_card_networks=supported_card_networks,
        )

        card_specific_features.additional_properties = d
        return card_specific_features

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
