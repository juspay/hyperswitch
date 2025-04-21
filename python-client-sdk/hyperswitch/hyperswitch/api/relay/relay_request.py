from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.relay_request import RelayRequest
from ...models.relay_response import RelayResponse
from ...types import Response


def _get_kwargs(
    *,
    body: RelayRequest,
    x_profile_id: str,
    x_idempotency_key: str,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}
    headers["X-Profile-Id"] = x_profile_id

    headers["X-Idempotency-Key"] = x_idempotency_key

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/relay",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RelayResponse]]:
    if response.status_code == 200:
        response_200 = RelayResponse.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, RelayResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: RelayRequest,
    x_profile_id: str,
    x_idempotency_key: str,
) -> Response[Union[Any, RelayResponse]]:
    """Relay - Create

     Creates a relay request.

    Args:
        x_profile_id (str):
        x_idempotency_key (str):
        body (RelayRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RelayResponse]]
    """

    kwargs = _get_kwargs(
        body=body,
        x_profile_id=x_profile_id,
        x_idempotency_key=x_idempotency_key,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: RelayRequest,
    x_profile_id: str,
    x_idempotency_key: str,
) -> Optional[Union[Any, RelayResponse]]:
    """Relay - Create

     Creates a relay request.

    Args:
        x_profile_id (str):
        x_idempotency_key (str):
        body (RelayRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RelayResponse]
    """

    return sync_detailed(
        client=client,
        body=body,
        x_profile_id=x_profile_id,
        x_idempotency_key=x_idempotency_key,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: RelayRequest,
    x_profile_id: str,
    x_idempotency_key: str,
) -> Response[Union[Any, RelayResponse]]:
    """Relay - Create

     Creates a relay request.

    Args:
        x_profile_id (str):
        x_idempotency_key (str):
        body (RelayRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RelayResponse]]
    """

    kwargs = _get_kwargs(
        body=body,
        x_profile_id=x_profile_id,
        x_idempotency_key=x_idempotency_key,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: RelayRequest,
    x_profile_id: str,
    x_idempotency_key: str,
) -> Optional[Union[Any, RelayResponse]]:
    """Relay - Create

     Creates a relay request.

    Args:
        x_profile_id (str):
        x_idempotency_key (str):
        body (RelayRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RelayResponse]
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
            x_profile_id=x_profile_id,
            x_idempotency_key=x_idempotency_key,
        )
    ).parsed
