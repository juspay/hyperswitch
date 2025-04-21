from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define

if TYPE_CHECKING:
    from ..models.payments_update_metadata_request_metadata import PaymentsUpdateMetadataRequestMetadata


T = TypeVar("T", bound="PaymentsUpdateMetadataRequest")


@_attrs_define
class PaymentsUpdateMetadataRequest:
    """
    Attributes:
        metadata (PaymentsUpdateMetadataRequestMetadata): Metadata is useful for storing additional, unstructured
            information on an object.
    """

    metadata: "PaymentsUpdateMetadataRequestMetadata"

    def to_dict(self) -> dict[str, Any]:
        metadata = self.metadata.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "metadata": metadata,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.payments_update_metadata_request_metadata import PaymentsUpdateMetadataRequestMetadata

        d = dict(src_dict)
        metadata = PaymentsUpdateMetadataRequestMetadata.from_dict(d.pop("metadata"))

        payments_update_metadata_request = cls(
            metadata=metadata,
        )

        return payments_update_metadata_request
