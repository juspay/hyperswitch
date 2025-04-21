from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.organization_update_request_metadata_type_0 import OrganizationUpdateRequestMetadataType0
    from ..models.organization_update_request_organization_details_type_0 import (
        OrganizationUpdateRequestOrganizationDetailsType0,
    )


T = TypeVar("T", bound="OrganizationUpdateRequest")


@_attrs_define
class OrganizationUpdateRequest:
    """
    Attributes:
        organization_name (Union[None, Unset, str]): Name of the organization
        organization_details (Union['OrganizationUpdateRequestOrganizationDetailsType0', None, Unset]): Details about
            the organization
        metadata (Union['OrganizationUpdateRequestMetadataType0', None, Unset]): Metadata is useful for storing
            additional, unstructured information on an object.
    """

    organization_name: Union[None, Unset, str] = UNSET
    organization_details: Union["OrganizationUpdateRequestOrganizationDetailsType0", None, Unset] = UNSET
    metadata: Union["OrganizationUpdateRequestMetadataType0", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.organization_update_request_metadata_type_0 import OrganizationUpdateRequestMetadataType0
        from ..models.organization_update_request_organization_details_type_0 import (
            OrganizationUpdateRequestOrganizationDetailsType0,
        )

        organization_name: Union[None, Unset, str]
        if isinstance(self.organization_name, Unset):
            organization_name = UNSET
        else:
            organization_name = self.organization_name

        organization_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.organization_details, Unset):
            organization_details = UNSET
        elif isinstance(self.organization_details, OrganizationUpdateRequestOrganizationDetailsType0):
            organization_details = self.organization_details.to_dict()
        else:
            organization_details = self.organization_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, OrganizationUpdateRequestMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if organization_name is not UNSET:
            field_dict["organization_name"] = organization_name
        if organization_details is not UNSET:
            field_dict["organization_details"] = organization_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.organization_update_request_metadata_type_0 import OrganizationUpdateRequestMetadataType0
        from ..models.organization_update_request_organization_details_type_0 import (
            OrganizationUpdateRequestOrganizationDetailsType0,
        )

        d = dict(src_dict)

        def _parse_organization_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        organization_name = _parse_organization_name(d.pop("organization_name", UNSET))

        def _parse_organization_details(
            data: object,
        ) -> Union["OrganizationUpdateRequestOrganizationDetailsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                organization_details_type_0 = OrganizationUpdateRequestOrganizationDetailsType0.from_dict(data)

                return organization_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationUpdateRequestOrganizationDetailsType0", None, Unset], data)

        organization_details = _parse_organization_details(d.pop("organization_details", UNSET))

        def _parse_metadata(data: object) -> Union["OrganizationUpdateRequestMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = OrganizationUpdateRequestMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationUpdateRequestMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        organization_update_request = cls(
            organization_name=organization_name,
            organization_details=organization_details,
            metadata=metadata,
        )

        return organization_update_request
