from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.success_rate_specificity_level import SuccessRateSpecificityLevel
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.current_block_threshold import CurrentBlockThreshold


T = TypeVar("T", bound="SuccessBasedRoutingConfigBody")


@_attrs_define
class SuccessBasedRoutingConfigBody:
    """
    Attributes:
        min_aggregates_size (Union[None, Unset, int]):
        default_success_rate (Union[None, Unset, float]):
        max_aggregates_size (Union[None, Unset, int]):
        current_block_threshold (Union['CurrentBlockThreshold', None, Unset]):
        specificity_level (Union[Unset, SuccessRateSpecificityLevel]):
    """

    min_aggregates_size: Union[None, Unset, int] = UNSET
    default_success_rate: Union[None, Unset, float] = UNSET
    max_aggregates_size: Union[None, Unset, int] = UNSET
    current_block_threshold: Union["CurrentBlockThreshold", None, Unset] = UNSET
    specificity_level: Union[Unset, SuccessRateSpecificityLevel] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.current_block_threshold import CurrentBlockThreshold

        min_aggregates_size: Union[None, Unset, int]
        if isinstance(self.min_aggregates_size, Unset):
            min_aggregates_size = UNSET
        else:
            min_aggregates_size = self.min_aggregates_size

        default_success_rate: Union[None, Unset, float]
        if isinstance(self.default_success_rate, Unset):
            default_success_rate = UNSET
        else:
            default_success_rate = self.default_success_rate

        max_aggregates_size: Union[None, Unset, int]
        if isinstance(self.max_aggregates_size, Unset):
            max_aggregates_size = UNSET
        else:
            max_aggregates_size = self.max_aggregates_size

        current_block_threshold: Union[None, Unset, dict[str, Any]]
        if isinstance(self.current_block_threshold, Unset):
            current_block_threshold = UNSET
        elif isinstance(self.current_block_threshold, CurrentBlockThreshold):
            current_block_threshold = self.current_block_threshold.to_dict()
        else:
            current_block_threshold = self.current_block_threshold

        specificity_level: Union[Unset, str] = UNSET
        if not isinstance(self.specificity_level, Unset):
            specificity_level = self.specificity_level.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if min_aggregates_size is not UNSET:
            field_dict["min_aggregates_size"] = min_aggregates_size
        if default_success_rate is not UNSET:
            field_dict["default_success_rate"] = default_success_rate
        if max_aggregates_size is not UNSET:
            field_dict["max_aggregates_size"] = max_aggregates_size
        if current_block_threshold is not UNSET:
            field_dict["current_block_threshold"] = current_block_threshold
        if specificity_level is not UNSET:
            field_dict["specificity_level"] = specificity_level

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.current_block_threshold import CurrentBlockThreshold

        d = dict(src_dict)

        def _parse_min_aggregates_size(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        min_aggregates_size = _parse_min_aggregates_size(d.pop("min_aggregates_size", UNSET))

        def _parse_default_success_rate(data: object) -> Union[None, Unset, float]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, float], data)

        default_success_rate = _parse_default_success_rate(d.pop("default_success_rate", UNSET))

        def _parse_max_aggregates_size(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        max_aggregates_size = _parse_max_aggregates_size(d.pop("max_aggregates_size", UNSET))

        def _parse_current_block_threshold(data: object) -> Union["CurrentBlockThreshold", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                current_block_threshold_type_1 = CurrentBlockThreshold.from_dict(data)

                return current_block_threshold_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CurrentBlockThreshold", None, Unset], data)

        current_block_threshold = _parse_current_block_threshold(d.pop("current_block_threshold", UNSET))

        _specificity_level = d.pop("specificity_level", UNSET)
        specificity_level: Union[Unset, SuccessRateSpecificityLevel]
        if isinstance(_specificity_level, Unset):
            specificity_level = UNSET
        else:
            specificity_level = SuccessRateSpecificityLevel(_specificity_level)

        success_based_routing_config_body = cls(
            min_aggregates_size=min_aggregates_size,
            default_success_rate=default_success_rate,
            max_aggregates_size=max_aggregates_size,
            current_block_threshold=current_block_threshold,
            specificity_level=specificity_level,
        )

        success_based_routing_config_body.additional_properties = d
        return success_based_routing_config_body

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
