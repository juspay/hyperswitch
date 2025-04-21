from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.dynamic_routing_features import DynamicRoutingFeatures
from ...models.routing_dictionary_record import RoutingDictionaryRecord
from ...types import UNSET, Response


def _get_kwargs(
    account_id: str,
    profile_id: str,
    *,
    enable: DynamicRoutingFeatures,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_enable = enable.value
    params["enable"] = json_enable

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/account/{account_id}/business_profile/{profile_id}/dynamic_routing/elimination/toggle",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    if response.status_code == 200:
        response_200 = RoutingDictionaryRecord.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if response.status_code == 403:
        response_403 = cast(Any, None)
        return response_403
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if response.status_code == 422:
        response_422 = cast(Any, None)
        return response_422
    if response.status_code == 500:
        response_500 = cast(Any, None)
        return response_500
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    enable: DynamicRoutingFeatures,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Toggle elimination routing for profile

     Create a elimination based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        enable (DynamicRoutingFeatures):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        enable=enable,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    enable: DynamicRoutingFeatures,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Toggle elimination routing for profile

     Create a elimination based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        enable (DynamicRoutingFeatures):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RoutingDictionaryRecord]
    """

    return sync_detailed(
        account_id=account_id,
        profile_id=profile_id,
        client=client,
        enable=enable,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    enable: DynamicRoutingFeatures,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Toggle elimination routing for profile

     Create a elimination based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        enable (DynamicRoutingFeatures):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        enable=enable,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    enable: DynamicRoutingFeatures,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Toggle elimination routing for profile

     Create a elimination based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        enable (DynamicRoutingFeatures):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RoutingDictionaryRecord]
    """

    return (
        await asyncio_detailed(
            account_id=account_id,
            profile_id=profile_id,
            client=client,
            enable=enable,
        )
    ).parsed
