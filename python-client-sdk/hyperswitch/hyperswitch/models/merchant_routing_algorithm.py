from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.transaction_type import TransactionType

if TYPE_CHECKING:
    from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
    from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
    from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
    from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3


T = TypeVar("T", bound="MerchantRoutingAlgorithm")


@_attrs_define
class MerchantRoutingAlgorithm:
    """Routing Algorithm specific to merchants

    Attributes:
        id (str):
        profile_id (str):
        name (str):
        description (str):
        algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3']): Routing Algorithm kind
        created_at (int):
        modified_at (int):
        algorithm_for (TransactionType):
    """

    id: str
    profile_id: str
    name: str
    description: str
    algorithm: Union["RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3"]
    created_at: int
    modified_at: int
    algorithm_for: TransactionType
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2

        id = self.id

        profile_id = self.profile_id

        name = self.name

        description = self.description

        algorithm: dict[str, Any]
        if isinstance(self.algorithm, RoutingAlgorithmType0):
            algorithm = self.algorithm.to_dict()
        elif isinstance(self.algorithm, RoutingAlgorithmType1):
            algorithm = self.algorithm.to_dict()
        elif isinstance(self.algorithm, RoutingAlgorithmType2):
            algorithm = self.algorithm.to_dict()
        else:
            algorithm = self.algorithm.to_dict()

        created_at = self.created_at

        modified_at = self.modified_at

        algorithm_for = self.algorithm_for.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "profile_id": profile_id,
                "name": name,
                "description": description,
                "algorithm": algorithm,
                "created_at": created_at,
                "modified_at": modified_at,
                "algorithm_for": algorithm_for,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3

        d = dict(src_dict)
        id = d.pop("id")

        profile_id = d.pop("profile_id")

        name = d.pop("name")

        description = d.pop("description")

        def _parse_algorithm(
            data: object,
        ) -> Union["RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3"]:
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
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_routing_algorithm_type_3 = RoutingAlgorithmType3.from_dict(data)

            return componentsschemas_routing_algorithm_type_3

        algorithm = _parse_algorithm(d.pop("algorithm"))

        created_at = d.pop("created_at")

        modified_at = d.pop("modified_at")

        algorithm_for = TransactionType(d.pop("algorithm_for"))

        merchant_routing_algorithm = cls(
            id=id,
            profile_id=profile_id,
            name=name,
            description=description,
            algorithm=algorithm,
            created_at=created_at,
            modified_at=modified_at,
            algorithm_for=algorithm_for,
        )

        merchant_routing_algorithm.additional_properties = d
        return merchant_routing_algorithm

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
