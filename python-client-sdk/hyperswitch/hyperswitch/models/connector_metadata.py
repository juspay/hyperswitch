from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.airwallex_data import AirwallexData
    from ..models.applepay_connector_metadata_request import ApplepayConnectorMetadataRequest
    from ..models.braintree_data import BraintreeData
    from ..models.noon_data import NoonData


T = TypeVar("T", bound="ConnectorMetadata")


@_attrs_define
class ConnectorMetadata:
    """Some connectors like Apple Pay, Airwallex and Noon might require some additional information, find specific details
    in the child attributes below.

        Attributes:
            apple_pay (Union['ApplepayConnectorMetadataRequest', None, Unset]):
            airwallex (Union['AirwallexData', None, Unset]):
            noon (Union['NoonData', None, Unset]):
            braintree (Union['BraintreeData', None, Unset]):
    """

    apple_pay: Union["ApplepayConnectorMetadataRequest", None, Unset] = UNSET
    airwallex: Union["AirwallexData", None, Unset] = UNSET
    noon: Union["NoonData", None, Unset] = UNSET
    braintree: Union["BraintreeData", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.airwallex_data import AirwallexData
        from ..models.applepay_connector_metadata_request import ApplepayConnectorMetadataRequest
        from ..models.braintree_data import BraintreeData
        from ..models.noon_data import NoonData

        apple_pay: Union[None, Unset, dict[str, Any]]
        if isinstance(self.apple_pay, Unset):
            apple_pay = UNSET
        elif isinstance(self.apple_pay, ApplepayConnectorMetadataRequest):
            apple_pay = self.apple_pay.to_dict()
        else:
            apple_pay = self.apple_pay

        airwallex: Union[None, Unset, dict[str, Any]]
        if isinstance(self.airwallex, Unset):
            airwallex = UNSET
        elif isinstance(self.airwallex, AirwallexData):
            airwallex = self.airwallex.to_dict()
        else:
            airwallex = self.airwallex

        noon: Union[None, Unset, dict[str, Any]]
        if isinstance(self.noon, Unset):
            noon = UNSET
        elif isinstance(self.noon, NoonData):
            noon = self.noon.to_dict()
        else:
            noon = self.noon

        braintree: Union[None, Unset, dict[str, Any]]
        if isinstance(self.braintree, Unset):
            braintree = UNSET
        elif isinstance(self.braintree, BraintreeData):
            braintree = self.braintree.to_dict()
        else:
            braintree = self.braintree

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if apple_pay is not UNSET:
            field_dict["apple_pay"] = apple_pay
        if airwallex is not UNSET:
            field_dict["airwallex"] = airwallex
        if noon is not UNSET:
            field_dict["noon"] = noon
        if braintree is not UNSET:
            field_dict["braintree"] = braintree

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.airwallex_data import AirwallexData
        from ..models.applepay_connector_metadata_request import ApplepayConnectorMetadataRequest
        from ..models.braintree_data import BraintreeData
        from ..models.noon_data import NoonData

        d = dict(src_dict)

        def _parse_apple_pay(data: object) -> Union["ApplepayConnectorMetadataRequest", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                apple_pay_type_1 = ApplepayConnectorMetadataRequest.from_dict(data)

                return apple_pay_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ApplepayConnectorMetadataRequest", None, Unset], data)

        apple_pay = _parse_apple_pay(d.pop("apple_pay", UNSET))

        def _parse_airwallex(data: object) -> Union["AirwallexData", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                airwallex_type_1 = AirwallexData.from_dict(data)

                return airwallex_type_1
            except:  # noqa: E722
                pass
            return cast(Union["AirwallexData", None, Unset], data)

        airwallex = _parse_airwallex(d.pop("airwallex", UNSET))

        def _parse_noon(data: object) -> Union["NoonData", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                noon_type_1 = NoonData.from_dict(data)

                return noon_type_1
            except:  # noqa: E722
                pass
            return cast(Union["NoonData", None, Unset], data)

        noon = _parse_noon(d.pop("noon", UNSET))

        def _parse_braintree(data: object) -> Union["BraintreeData", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                braintree_type_1 = BraintreeData.from_dict(data)

                return braintree_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BraintreeData", None, Unset], data)

        braintree = _parse_braintree(d.pop("braintree", UNSET))

        connector_metadata = cls(
            apple_pay=apple_pay,
            airwallex=airwallex,
            noon=noon,
            braintree=braintree,
        )

        connector_metadata.additional_properties = d
        return connector_metadata

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
