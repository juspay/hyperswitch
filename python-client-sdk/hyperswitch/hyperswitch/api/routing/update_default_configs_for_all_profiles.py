from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.profile_default_routing_config import ProfileDefaultRoutingConfig
from ...models.routable_connector_choice import RoutableConnectorChoice
from ...types import Response


def _get_kwargs(
    profile_id: str,
    *,
    body: list["RoutableConnectorChoice"],
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/routing/default/profile/{profile_id}",
    }

    _body = []
    for body_item_data in body:
        body_item = body_item_data.to_dict()
        _body.append(body_item)

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, ProfileDefaultRoutingConfig]]:
    if response.status_code == 200:
        response_200 = ProfileDefaultRoutingConfig.from_dict(response.json())

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
) -> Response[Union[Any, ProfileDefaultRoutingConfig]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: list["RoutableConnectorChoice"],
) -> Response[Union[Any, ProfileDefaultRoutingConfig]]:
    """Routing - Update Default For Profile

     Update default config for profiles

    Args:
        profile_id (str):
        body (list['RoutableConnectorChoice']):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, ProfileDefaultRoutingConfig]]
    """

    kwargs = _get_kwargs(
        profile_id=profile_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: list["RoutableConnectorChoice"],
) -> Optional[Union[Any, ProfileDefaultRoutingConfig]]:
    """Routing - Update Default For Profile

     Update default config for profiles

    Args:
        profile_id (str):
        body (list['RoutableConnectorChoice']):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, ProfileDefaultRoutingConfig]
    """

    return sync_detailed(
        profile_id=profile_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: list["RoutableConnectorChoice"],
) -> Response[Union[Any, ProfileDefaultRoutingConfig]]:
    """Routing - Update Default For Profile

     Update default config for profiles

    Args:
        profile_id (str):
        body (list['RoutableConnectorChoice']):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, ProfileDefaultRoutingConfig]]
    """

    kwargs = _get_kwargs(
        profile_id=profile_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: list["RoutableConnectorChoice"],
) -> Optional[Union[Any, ProfileDefaultRoutingConfig]]:
    """Routing - Update Default For Profile

     Update default config for profiles

    Args:
        profile_id (str):
        body (list['RoutableConnectorChoice']):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, ProfileDefaultRoutingConfig]
    """

    return (
        await asyncio_detailed(
            profile_id=profile_id,
            client=client,
            body=body,
        )
    ).parsed
