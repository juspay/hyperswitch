from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
    from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
    from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
    from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3


T = TypeVar("T", bound="RoutingConfigRequest")


@_attrs_define
class RoutingConfigRequest:
    """
    Attributes:
        name (Union[None, Unset, str]):
        description (Union[None, Unset, str]):
        algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3', None, Unset]):
        profile_id (Union[None, Unset, str]):
    """

    name: Union[None, Unset, str] = UNSET
    description: Union[None, Unset, str] = UNSET
    algorithm: Union[
        "RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3", None, Unset
    ] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3

        name: Union[None, Unset, str]
        if isinstance(self.name, Unset):
            name = UNSET
        else:
            name = self.name

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.algorithm, Unset):
            algorithm = UNSET
        elif isinstance(self.algorithm, RoutingAlgorithmType0):
            algorithm = self.algorithm.to_dict()
        elif isinstance(self.algorithm, RoutingAlgorithmType1):
            algorithm = self.algorithm.to_dict()
        elif isinstance(self.algorithm, RoutingAlgorithmType2):
            algorithm = self.algorithm.to_dict()
        elif isinstance(self.algorithm, RoutingAlgorithmType3):
            algorithm = self.algorithm.to_dict()
        else:
            algorithm = self.algorithm

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if name is not UNSET:
            field_dict["name"] = name
        if description is not UNSET:
            field_dict["description"] = description
        if algorithm is not UNSET:
            field_dict["algorithm"] = algorithm
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3

        d = dict(src_dict)

        def _parse_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        name = _parse_name(d.pop("name", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_algorithm(
            data: object,
        ) -> Union[
            "RoutingAlgorithmType0",
            "RoutingAlgorithmType1",
            "RoutingAlgorithmType2",
            "RoutingAlgorithmType3",
            None,
            Unset,
        ]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_0 = RoutingAlgorithmType0.from_dict(data)

                return componentsschemas_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_1 = RoutingAlgorithmType1.from_dict(data)

                return componentsschemas_routing_algorithm_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_2 = RoutingAlgorithmType2.from_dict(data)

                return componentsschemas_routing_algorithm_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_3 = RoutingAlgorithmType3.from_dict(data)

                return componentsschemas_routing_algorithm_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "RoutingAlgorithmType0",
                    "RoutingAlgorithmType1",
                    "RoutingAlgorithmType2",
                    "RoutingAlgorithmType3",
                    None,
                    Unset,
                ],
                data,
            )

        algorithm = _parse_algorithm(d.pop("algorithm", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        routing_config_request = cls(
            name=name,
            description=description,
            algorithm=algorithm,
            profile_id=profile_id,
        )

        routing_config_request.additional_properties = d
        return routing_config_request

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
