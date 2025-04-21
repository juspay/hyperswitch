from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.surcharge_details_response import SurchargeDetailsResponse


T = TypeVar("T", bound="CardNetworkTypes")


@_attrs_define
class CardNetworkTypes:
    """
    Attributes:
        eligible_connectors (list[str]): The list of eligible connectors for a given card network Example: ['stripe',
            'adyen'].
        card_network (Union[CardNetwork, None, Unset]):
        surcharge_details (Union['SurchargeDetailsResponse', None, Unset]):
    """

    eligible_connectors: list[str]
    card_network: Union[CardNetwork, None, Unset] = UNSET
    surcharge_details: Union["SurchargeDetailsResponse", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.surcharge_details_response import SurchargeDetailsResponse

        eligible_connectors = self.eligible_connectors

        card_network: Union[None, Unset, str]
        if isinstance(self.card_network, Unset):
            card_network = UNSET
        elif isinstance(self.card_network, CardNetwork):
            card_network = self.card_network.value
        else:
            card_network = self.card_network

        surcharge_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.surcharge_details, Unset):
            surcharge_details = UNSET
        elif isinstance(self.surcharge_details, SurchargeDetailsResponse):
            surcharge_details = self.surcharge_details.to_dict()
        else:
            surcharge_details = self.surcharge_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "eligible_connectors": eligible_connectors,
            }
        )
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if surcharge_details is not UNSET:
            field_dict["surcharge_details"] = surcharge_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.surcharge_details_response import SurchargeDetailsResponse

        d = dict(src_dict)
        eligible_connectors = cast(list[str], d.pop("eligible_connectors"))

        def _parse_card_network(data: object) -> Union[CardNetwork, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                card_network_type_1 = CardNetwork(data)

                return card_network_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CardNetwork, None, Unset], data)

        card_network = _parse_card_network(d.pop("card_network", UNSET))

        def _parse_surcharge_details(data: object) -> Union["SurchargeDetailsResponse", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                surcharge_details_type_1 = SurchargeDetailsResponse.from_dict(data)

                return surcharge_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SurchargeDetailsResponse", None, Unset], data)

        surcharge_details = _parse_surcharge_details(d.pop("surcharge_details", UNSET))

        card_network_types = cls(
            eligible_connectors=eligible_connectors,
            card_network=card_network,
            surcharge_details=surcharge_details,
        )

        card_network_types.additional_properties = d
        return card_network_types

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
