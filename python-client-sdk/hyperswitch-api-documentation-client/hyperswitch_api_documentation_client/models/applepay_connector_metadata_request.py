from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.session_token_info import SessionTokenInfo


T = TypeVar("T", bound="ApplepayConnectorMetadataRequest")


@_attrs_define
class ApplepayConnectorMetadataRequest:
    """
    Attributes:
        session_token_data (Union['SessionTokenInfo', None, Unset]):
    """

    session_token_data: Union["SessionTokenInfo", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.session_token_info import SessionTokenInfo

        session_token_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.session_token_data, Unset):
            session_token_data = UNSET
        elif isinstance(self.session_token_data, SessionTokenInfo):
            session_token_data = self.session_token_data.to_dict()
        else:
            session_token_data = self.session_token_data

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if session_token_data is not UNSET:
            field_dict["session_token_data"] = session_token_data

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.session_token_info import SessionTokenInfo

        d = dict(src_dict)

        def _parse_session_token_data(data: object) -> Union["SessionTokenInfo", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                session_token_data_type_1 = SessionTokenInfo.from_dict(data)

                return session_token_data_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SessionTokenInfo", None, Unset], data)

        session_token_data = _parse_session_token_data(d.pop("session_token_data", UNSET))

        applepay_connector_metadata_request = cls(
            session_token_data=session_token_data,
        )

        applepay_connector_metadata_request.additional_properties = d
        return applepay_connector_metadata_request

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
