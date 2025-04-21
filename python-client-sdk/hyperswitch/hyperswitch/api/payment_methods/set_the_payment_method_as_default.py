from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.customer_default_payment_method_response import CustomerDefaultPaymentMethodResponse
from ...types import Response


def _get_kwargs(
    customer_id: str,
    payment_method_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/{customer_id}/payment_methods/{payment_method_id}/default",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    if response.status_code == 200:
        response_200 = CustomerDefaultPaymentMethodResponse.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    customer_id: str,
    payment_method_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    """Payment Method - Set Default Payment Method for Customer

     Set the Payment Method as Default for the Customer.

    Args:
        customer_id (str):
        payment_method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, CustomerDefaultPaymentMethodResponse]]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        payment_method_id=payment_method_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    customer_id: str,
    payment_method_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    """Payment Method - Set Default Payment Method for Customer

     Set the Payment Method as Default for the Customer.

    Args:
        customer_id (str):
        payment_method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, CustomerDefaultPaymentMethodResponse]
    """

    return sync_detailed(
        customer_id=customer_id,
        payment_method_id=payment_method_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    customer_id: str,
    payment_method_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    """Payment Method - Set Default Payment Method for Customer

     Set the Payment Method as Default for the Customer.

    Args:
        customer_id (str):
        payment_method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, CustomerDefaultPaymentMethodResponse]]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        payment_method_id=payment_method_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    customer_id: str,
    payment_method_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, CustomerDefaultPaymentMethodResponse]]:
    """Payment Method - Set Default Payment Method for Customer

     Set the Payment Method as Default for the Customer.

    Args:
        customer_id (str):
        payment_method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, CustomerDefaultPaymentMethodResponse]
    """

    return (
        await asyncio_detailed(
            customer_id=customer_id,
            payment_method_id=payment_method_id,
            client=client,
        )
    ).parsed
