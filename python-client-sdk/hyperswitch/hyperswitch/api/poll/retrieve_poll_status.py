from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.poll_response import PollResponse
from ...types import Response


def _get_kwargs(
    poll_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/poll/status/{poll_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PollResponse]]:
    if response.status_code == 200:
        response_200 = PollResponse.from_dict(response.json())

        return response_200
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, PollResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    poll_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, PollResponse]]:
    """Poll - Retrieve Poll Status

    Args:
        poll_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PollResponse]]
    """

    kwargs = _get_kwargs(
        poll_id=poll_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    poll_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, PollResponse]]:
    """Poll - Retrieve Poll Status

    Args:
        poll_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PollResponse]
    """

    return sync_detailed(
        poll_id=poll_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    poll_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, PollResponse]]:
    """Poll - Retrieve Poll Status

    Args:
        poll_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PollResponse]]
    """

    kwargs = _get_kwargs(
        poll_id=poll_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    poll_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, PollResponse]]:
    """Poll - Retrieve Poll Status

    Args:
        poll_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PollResponse]
    """

    return (
        await asyncio_detailed(
            poll_id=poll_id,
            client=client,
        )
    ).parsed
