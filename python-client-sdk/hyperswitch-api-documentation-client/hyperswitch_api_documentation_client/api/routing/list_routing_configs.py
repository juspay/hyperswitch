from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.routing_dictionary import RoutingDictionary
from ...models.routing_dictionary_record import RoutingDictionaryRecord
from ...types import UNSET, Response, Unset


def _get_kwargs(
    *,
    limit: Union[None, Unset, int] = UNSET,
    offset: Union[None, Unset, int] = UNSET,
    profile_id: Union[None, Unset, str] = UNSET,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_limit: Union[None, Unset, int]
    if isinstance(limit, Unset):
        json_limit = UNSET
    else:
        json_limit = limit
    params["limit"] = json_limit

    json_offset: Union[None, Unset, int]
    if isinstance(offset, Unset):
        json_offset = UNSET
    else:
        json_offset = offset
    params["offset"] = json_offset

    json_profile_id: Union[None, Unset, str]
    if isinstance(profile_id, Unset):
        json_profile_id = UNSET
    else:
        json_profile_id = profile_id
    params["profile_id"] = json_profile_id

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/routing",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    if response.status_code == 200:

        def _parse_response_200(data: object) -> Union["RoutingDictionary", list["RoutingDictionaryRecord"]]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_kind_type_0 = RoutingDictionary.from_dict(data)

                return componentsschemas_routing_kind_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, list):
                raise TypeError()
            componentsschemas_routing_kind_type_1 = []
            _componentsschemas_routing_kind_type_1 = data
            for componentsschemas_routing_kind_type_1_item_data in _componentsschemas_routing_kind_type_1:
                componentsschemas_routing_kind_type_1_item = RoutingDictionaryRecord.from_dict(
                    componentsschemas_routing_kind_type_1_item_data
                )

                componentsschemas_routing_kind_type_1.append(componentsschemas_routing_kind_type_1_item)

            return componentsschemas_routing_kind_type_1

        response_200 = _parse_response_200(response.json())

        return response_200
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if response.status_code == 500:
        response_500 = cast(Any, None)
        return response_500
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    offset: Union[None, Unset, int] = UNSET,
    profile_id: Union[None, Unset, str] = UNSET,
) -> Response[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    """Routing - List

     List all routing configs

    Args:
        limit (Union[None, Unset, int]):
        offset (Union[None, Unset, int]):
        profile_id (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, Union['RoutingDictionary', list['RoutingDictionaryRecord']]]]
    """

    kwargs = _get_kwargs(
        limit=limit,
        offset=offset,
        profile_id=profile_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    offset: Union[None, Unset, int] = UNSET,
    profile_id: Union[None, Unset, str] = UNSET,
) -> Optional[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    """Routing - List

     List all routing configs

    Args:
        limit (Union[None, Unset, int]):
        offset (Union[None, Unset, int]):
        profile_id (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, Union['RoutingDictionary', list['RoutingDictionaryRecord']]]
    """

    return sync_detailed(
        client=client,
        limit=limit,
        offset=offset,
        profile_id=profile_id,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    offset: Union[None, Unset, int] = UNSET,
    profile_id: Union[None, Unset, str] = UNSET,
) -> Response[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    """Routing - List

     List all routing configs

    Args:
        limit (Union[None, Unset, int]):
        offset (Union[None, Unset, int]):
        profile_id (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, Union['RoutingDictionary', list['RoutingDictionaryRecord']]]]
    """

    kwargs = _get_kwargs(
        limit=limit,
        offset=offset,
        profile_id=profile_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    offset: Union[None, Unset, int] = UNSET,
    profile_id: Union[None, Unset, str] = UNSET,
) -> Optional[Union[Any, Union["RoutingDictionary", list["RoutingDictionaryRecord"]]]]:
    """Routing - List

     List all routing configs

    Args:
        limit (Union[None, Unset, int]):
        offset (Union[None, Unset, int]):
        profile_id (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, Union['RoutingDictionary', list['RoutingDictionaryRecord']]]
    """

    return (
        await asyncio_detailed(
            client=client,
            limit=limit,
            offset=offset,
            profile_id=profile_id,
        )
    ).parsed
