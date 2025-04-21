from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details_connector_account_details_type_0 import (
        MerchantConnectorDetailsConnectorAccountDetailsType0,
    )
    from ..models.merchant_connector_details_metadata_type_0 import MerchantConnectorDetailsMetadataType0


T = TypeVar("T", bound="MerchantConnectorDetails")


@_attrs_define
class MerchantConnectorDetails:
    """
    Attributes:
        connector_account_details (Union['MerchantConnectorDetailsConnectorAccountDetailsType0', None, Unset]): Account
            details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up
            to 500 characters long. Useful for storing additional, structured information on an object.
        metadata (Union['MerchantConnectorDetailsMetadataType0', None, Unset]): Metadata is useful for storing
            additional, unstructured information on an object.
    """

    connector_account_details: Union["MerchantConnectorDetailsConnectorAccountDetailsType0", None, Unset] = UNSET
    metadata: Union["MerchantConnectorDetailsMetadataType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details_connector_account_details_type_0 import (
            MerchantConnectorDetailsConnectorAccountDetailsType0,
        )
        from ..models.merchant_connector_details_metadata_type_0 import MerchantConnectorDetailsMetadataType0

        connector_account_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.connector_account_details, Unset):
            connector_account_details = UNSET
        elif isinstance(self.connector_account_details, MerchantConnectorDetailsConnectorAccountDetailsType0):
            connector_account_details = self.connector_account_details.to_dict()
        else:
            connector_account_details = self.connector_account_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, MerchantConnectorDetailsMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if connector_account_details is not UNSET:
            field_dict["connector_account_details"] = connector_account_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details_connector_account_details_type_0 import (
            MerchantConnectorDetailsConnectorAccountDetailsType0,
        )
        from ..models.merchant_connector_details_metadata_type_0 import MerchantConnectorDetailsMetadataType0

        d = dict(src_dict)

        def _parse_connector_account_details(
            data: object,
        ) -> Union["MerchantConnectorDetailsConnectorAccountDetailsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                connector_account_details_type_0 = MerchantConnectorDetailsConnectorAccountDetailsType0.from_dict(data)

                return connector_account_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsConnectorAccountDetailsType0", None, Unset], data)

        connector_account_details = _parse_connector_account_details(d.pop("connector_account_details", UNSET))

        def _parse_metadata(data: object) -> Union["MerchantConnectorDetailsMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = MerchantConnectorDetailsMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        merchant_connector_details = cls(
            connector_account_details=connector_account_details,
            metadata=metadata,
        )

        merchant_connector_details.additional_properties = d
        return merchant_connector_details

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
