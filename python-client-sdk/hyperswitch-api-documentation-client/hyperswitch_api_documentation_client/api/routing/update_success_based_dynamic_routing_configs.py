from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.routing_dictionary_record import RoutingDictionaryRecord
from ...models.success_based_routing_config import SuccessBasedRoutingConfig
from ...types import Response


def _get_kwargs(
    account_id: str,
    profile_id: str,
    algorithm_id: str,
    *,
    body: SuccessBasedRoutingConfig,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "patch",
        "url": f"/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/config/{algorithm_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
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
    algorithm_id: str,
    *,
    client: AuthenticatedClient,
    body: SuccessBasedRoutingConfig,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Update success based dynamic routing config for profile

     Update success based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        algorithm_id (str):
        body (SuccessBasedRoutingConfig):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        algorithm_id=algorithm_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    account_id: str,
    profile_id: str,
    algorithm_id: str,
    *,
    client: AuthenticatedClient,
    body: SuccessBasedRoutingConfig,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Update success based dynamic routing config for profile

     Update success based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        algorithm_id (str):
        body (SuccessBasedRoutingConfig):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RoutingDictionaryRecord]
    """

    return sync_detailed(
        account_id=account_id,
        profile_id=profile_id,
        algorithm_id=algorithm_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    profile_id: str,
    algorithm_id: str,
    *,
    client: AuthenticatedClient,
    body: SuccessBasedRoutingConfig,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Update success based dynamic routing config for profile

     Update success based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        algorithm_id (str):
        body (SuccessBasedRoutingConfig):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        algorithm_id=algorithm_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    profile_id: str,
    algorithm_id: str,
    *,
    client: AuthenticatedClient,
    body: SuccessBasedRoutingConfig,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Update success based dynamic routing config for profile

     Update success based dynamic routing algorithm

    Args:
        account_id (str):
        profile_id (str):
        algorithm_id (str):
        body (SuccessBasedRoutingConfig):

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
            algorithm_id=algorithm_id,
            client=client,
            body=body,
        )
    ).parsed
