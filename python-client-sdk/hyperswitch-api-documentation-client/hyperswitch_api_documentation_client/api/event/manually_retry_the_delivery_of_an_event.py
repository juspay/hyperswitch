from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.event_retrieve_response import EventRetrieveResponse
from ...types import Response


def _get_kwargs(
    merchant_id: str,
    event_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/events/{merchant_id}/{event_id}/retry",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[EventRetrieveResponse]:
    if response.status_code == 200:
        response_200 = EventRetrieveResponse.from_dict(response.json())

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[EventRetrieveResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    merchant_id: str,
    event_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[EventRetrieveResponse]:
    """Events - Manual Retry

     Manually retry the delivery of the specified Event.

    Args:
        merchant_id (str):
        event_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[EventRetrieveResponse]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        event_id=event_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    merchant_id: str,
    event_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[EventRetrieveResponse]:
    """Events - Manual Retry

     Manually retry the delivery of the specified Event.

    Args:
        merchant_id (str):
        event_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        EventRetrieveResponse
    """

    return sync_detailed(
        merchant_id=merchant_id,
        event_id=event_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    merchant_id: str,
    event_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[EventRetrieveResponse]:
    """Events - Manual Retry

     Manually retry the delivery of the specified Event.

    Args:
        merchant_id (str):
        event_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[EventRetrieveResponse]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        event_id=event_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    merchant_id: str,
    event_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[EventRetrieveResponse]:
    """Events - Manual Retry

     Manually retry the delivery of the specified Event.

    Args:
        merchant_id (str):
        event_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        EventRetrieveResponse
    """

    return (
        await asyncio_detailed(
            merchant_id=merchant_id,
            event_id=event_id,
            client=client,
        )
    ).parsed
