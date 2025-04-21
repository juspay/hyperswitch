from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payout_create_response import PayoutCreateResponse
from ...models.payout_fulfill_request import PayoutFulfillRequest
from ...types import Response


def _get_kwargs(
    payout_id: str,
    *,
    body: PayoutFulfillRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/payouts/{payout_id}/fulfill",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PayoutCreateResponse]]:
    if response.status_code == 200:
        response_200 = PayoutCreateResponse.from_dict(response.json())

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
) -> Response[Union[Any, PayoutCreateResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    body: PayoutFulfillRequest,
) -> Response[Union[Any, PayoutCreateResponse]]:
    """Payouts - Fulfill

    Args:
        payout_id (str):
        body (PayoutFulfillRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutCreateResponse]]
    """

    kwargs = _get_kwargs(
        payout_id=payout_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    body: PayoutFulfillRequest,
) -> Optional[Union[Any, PayoutCreateResponse]]:
    """Payouts - Fulfill

    Args:
        payout_id (str):
        body (PayoutFulfillRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutCreateResponse]
    """

    return sync_detailed(
        payout_id=payout_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    body: PayoutFulfillRequest,
) -> Response[Union[Any, PayoutCreateResponse]]:
    """Payouts - Fulfill

    Args:
        payout_id (str):
        body (PayoutFulfillRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutCreateResponse]]
    """

    kwargs = _get_kwargs(
        payout_id=payout_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    body: PayoutFulfillRequest,
) -> Optional[Union[Any, PayoutCreateResponse]]:
    """Payouts - Fulfill

    Args:
        payout_id (str):
        body (PayoutFulfillRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutCreateResponse]
    """

    return (
        await asyncio_detailed(
            payout_id=payout_id,
            client=client,
            body=body,
        )
    ).parsed
