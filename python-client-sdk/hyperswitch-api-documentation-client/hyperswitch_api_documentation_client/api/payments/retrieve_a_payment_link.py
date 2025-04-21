from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.retrieve_payment_link_request import RetrievePaymentLinkRequest
from ...models.retrieve_payment_link_response import RetrievePaymentLinkResponse
from ...types import Response


def _get_kwargs(
    payment_link_id: str,
    *,
    body: RetrievePaymentLinkRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/payment_link/{payment_link_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RetrievePaymentLinkResponse]]:
    if response.status_code == 200:
        response_200 = RetrievePaymentLinkResponse.from_dict(response.json())

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
) -> Response[Union[Any, RetrievePaymentLinkResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    payment_link_id: str,
    *,
    client: AuthenticatedClient,
    body: RetrievePaymentLinkRequest,
) -> Response[Union[Any, RetrievePaymentLinkResponse]]:
    """Payments Link - Retrieve

     To retrieve the properties of a Payment Link. This may be used to get the status of a previously
    initiated payment or next action for an ongoing payment

    Args:
        payment_link_id (str):
        body (RetrievePaymentLinkRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RetrievePaymentLinkResponse]]
    """

    kwargs = _get_kwargs(
        payment_link_id=payment_link_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    payment_link_id: str,
    *,
    client: AuthenticatedClient,
    body: RetrievePaymentLinkRequest,
) -> Optional[Union[Any, RetrievePaymentLinkResponse]]:
    """Payments Link - Retrieve

     To retrieve the properties of a Payment Link. This may be used to get the status of a previously
    initiated payment or next action for an ongoing payment

    Args:
        payment_link_id (str):
        body (RetrievePaymentLinkRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RetrievePaymentLinkResponse]
    """

    return sync_detailed(
        payment_link_id=payment_link_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    payment_link_id: str,
    *,
    client: AuthenticatedClient,
    body: RetrievePaymentLinkRequest,
) -> Response[Union[Any, RetrievePaymentLinkResponse]]:
    """Payments Link - Retrieve

     To retrieve the properties of a Payment Link. This may be used to get the status of a previously
    initiated payment or next action for an ongoing payment

    Args:
        payment_link_id (str):
        body (RetrievePaymentLinkRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RetrievePaymentLinkResponse]]
    """

    kwargs = _get_kwargs(
        payment_link_id=payment_link_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    payment_link_id: str,
    *,
    client: AuthenticatedClient,
    body: RetrievePaymentLinkRequest,
) -> Optional[Union[Any, RetrievePaymentLinkResponse]]:
    """Payments Link - Retrieve

     To retrieve the properties of a Payment Link. This may be used to get the status of a previously
    initiated payment or next action for an ongoing payment

    Args:
        payment_link_id (str):
        body (RetrievePaymentLinkRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RetrievePaymentLinkResponse]
    """

    return (
        await asyncio_detailed(
            payment_link_id=payment_link_id,
            client=client,
            body=body,
        )
    ).parsed
