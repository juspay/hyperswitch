from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.redirect_response_json_payload_type_0 import RedirectResponseJsonPayloadType0


T = TypeVar("T", bound="RedirectResponse")


@_attrs_define
class RedirectResponse:
    """
    Attributes:
        param (Union[None, Unset, str]):
        json_payload (Union['RedirectResponseJsonPayloadType0', None, Unset]):
    """

    param: Union[None, Unset, str] = UNSET
    json_payload: Union["RedirectResponseJsonPayloadType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.redirect_response_json_payload_type_0 import RedirectResponseJsonPayloadType0

        param: Union[None, Unset, str]
        if isinstance(self.param, Unset):
            param = UNSET
        else:
            param = self.param

        json_payload: Union[None, Unset, dict[str, Any]]
        if isinstance(self.json_payload, Unset):
            json_payload = UNSET
        elif isinstance(self.json_payload, RedirectResponseJsonPayloadType0):
            json_payload = self.json_payload.to_dict()
        else:
            json_payload = self.json_payload

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if param is not UNSET:
            field_dict["param"] = param
        if json_payload is not UNSET:
            field_dict["json_payload"] = json_payload

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.redirect_response_json_payload_type_0 import RedirectResponseJsonPayloadType0

        d = dict(src_dict)

        def _parse_param(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        param = _parse_param(d.pop("param", UNSET))

        def _parse_json_payload(data: object) -> Union["RedirectResponseJsonPayloadType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                json_payload_type_0 = RedirectResponseJsonPayloadType0.from_dict(data)

                return json_payload_type_0
            except:  # noqa: E722
                pass
            return cast(Union["RedirectResponseJsonPayloadType0", None, Unset], data)

        json_payload = _parse_json_payload(d.pop("json_payload", UNSET))

        redirect_response = cls(
            param=param,
            json_payload=json_payload,
        )

        redirect_response.additional_properties = d
        return redirect_response

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
