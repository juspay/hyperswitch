from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.routing_algorithm_kind import RoutingAlgorithmKind
from ..models.transaction_type import TransactionType
from ..types import UNSET, Unset

T = TypeVar("T", bound="RoutingDictionaryRecord")


@_attrs_define
class RoutingDictionaryRecord:
    """
    Attributes:
        id (str):
        profile_id (str):
        name (str):
        kind (RoutingAlgorithmKind):
        description (str):
        created_at (int):
        modified_at (int):
        algorithm_for (Union[None, TransactionType, Unset]):
    """

    id: str
    profile_id: str
    name: str
    kind: RoutingAlgorithmKind
    description: str
    created_at: int
    modified_at: int
    algorithm_for: Union[None, TransactionType, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        profile_id = self.profile_id

        name = self.name

        kind = self.kind.value

        description = self.description

        created_at = self.created_at

        modified_at = self.modified_at

        algorithm_for: Union[None, Unset, str]
        if isinstance(self.algorithm_for, Unset):
            algorithm_for = UNSET
        elif isinstance(self.algorithm_for, TransactionType):
            algorithm_for = self.algorithm_for.value
        else:
            algorithm_for = self.algorithm_for

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "profile_id": profile_id,
                "name": name,
                "kind": kind,
                "description": description,
                "created_at": created_at,
                "modified_at": modified_at,
            }
        )
        if algorithm_for is not UNSET:
            field_dict["algorithm_for"] = algorithm_for

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        profile_id = d.pop("profile_id")

        name = d.pop("name")

        kind = RoutingAlgorithmKind(d.pop("kind"))

        description = d.pop("description")

        created_at = d.pop("created_at")

        modified_at = d.pop("modified_at")

        def _parse_algorithm_for(data: object) -> Union[None, TransactionType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                algorithm_for_type_1 = TransactionType(data)

                return algorithm_for_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, TransactionType, Unset], data)

        algorithm_for = _parse_algorithm_for(d.pop("algorithm_for", UNSET))

        routing_dictionary_record = cls(
            id=id,
            profile_id=profile_id,
            name=name,
            kind=kind,
            description=description,
            created_at=created_at,
            modified_at=modified_at,
            algorithm_for=algorithm_for,
        )

        routing_dictionary_record.additional_properties = d
        return routing_dictionary_record

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
