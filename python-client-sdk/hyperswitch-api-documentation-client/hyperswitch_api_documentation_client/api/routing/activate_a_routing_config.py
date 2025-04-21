from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.routing_dictionary_record import RoutingDictionaryRecord
from ...types import Response


def _get_kwargs(
    routing_algorithm_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/routing/{routing_algorithm_id}/activate",
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
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    routing_algorithm_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Activate config

     Activate a routing config

    Args:
        routing_algorithm_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        routing_algorithm_id=routing_algorithm_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    routing_algorithm_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Activate config

     Activate a routing config

    Args:
        routing_algorithm_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RoutingDictionaryRecord]
    """

    return sync_detailed(
        routing_algorithm_id=routing_algorithm_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    routing_algorithm_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Activate config

     Activate a routing config

    Args:
        routing_algorithm_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RoutingDictionaryRecord]]
    """

    kwargs = _get_kwargs(
        routing_algorithm_id=routing_algorithm_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    routing_algorithm_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RoutingDictionaryRecord]]:
    """Routing - Activate config

     Activate a routing config

    Args:
        routing_algorithm_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RoutingDictionaryRecord]
    """

    return (
        await asyncio_detailed(
            routing_algorithm_id=routing_algorithm_id,
            client=client,
        )
    ).parsed
