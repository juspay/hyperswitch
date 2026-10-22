---
name: hyperswitch-react-sdk
description: Use this skill when the user asks about "Hyperswitch React SDK", "HyperElements", "PaymentElement", "loadHyper", "@juspay-tech/hyper-js", "Hyperswitch frontend integration", "embed a payment form", "React payment component", "Unified Checkout in React", "styling the payment form", "confirmPayment React", or needs to build a client-side checkout using Hyperswitch.
version: 1.0.0
tags: [hyperswitch, react, sdk, frontend, checkout, hyper-js]
---

# React SDK Integration

## Overview

The Hyperswitch React SDK (`@juspay-tech/hyper-js` + `@juspay-tech/react-hyper-js`) embeds a secure, customizable payment form directly in your React application. The form handles card input, wallet buttons, 3DS redirects, and bank payment methods — all without PCI scope falling on your frontend.

## Prerequisites

- A Hyperswitch account with a configured connector
- A server endpoint that creates a Hyperswitch payment and returns the `client_secret`
- Node.js 16+ and a React 16.8+ application

---

## Installation

```bash
npm install @juspay-tech/hyper-js @juspay-tech/react-hyper-js
# or
yarn add @juspay-tech/hyper-js @juspay-tech/react-hyper-js
```

---

## Architecture

```
Your React App
     ↓
HyperElements (wraps SDK context)
     ↓
PaymentElement (renders payment form)
     ↓
confirmPayment() (submits payment)
     ↓
Hyperswitch SDK → Hyperswitch API → Connector
```

---

## Step 1: Create a Payment on Your Server

Your server creates the payment and returns the `client_secret` to the frontend:

```javascript
// server.js (Node/Express example)
app.post('/create-payment', async (req, res) => {
  const response = await fetch('https://sandbox.hyperswitch.io/payments', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'api-key': process.env.HYPERSWITCH_API_KEY,
    },
    body: JSON.stringify({
      amount: req.body.amount,
      currency: 'USD',
      confirm: false,
      customer_id: req.user.id,
      return_url: `${process.env.APP_URL}/payment/complete`,
    }),
  });

  const payment = await response.json();
  res.json({ clientSecret: payment.client_secret });
});
```

---

## Step 2: Load Hyperswitch SDK

```javascript
// hyper.js — load once at app start
import { loadHyper } from '@juspay-tech/hyper-js';

const hyperPromise = loadHyper(process.env.REACT_APP_HYPERSWITCH_PUBLISHABLE_KEY, {
  customBackendUrl: 'https://sandbox.hyperswitch.io',
});

export default hyperPromise;
```

Your **publishable key** (also called `client_key`) is in **Hyperswitch Dashboard → Developers → API Keys**.

---

## Step 3: Build the Checkout Component

```jsx
// CheckoutPage.jsx
import React, { useState, useEffect } from 'react';
import { Elements } from '@juspay-tech/react-hyper-js';
import PaymentForm from './PaymentForm';
import hyperPromise from './hyper';

export default function CheckoutPage({ amount }) {
  const [clientSecret, setClientSecret] = useState(null);

  useEffect(() => {
    fetch('/create-payment', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ amount }),
    })
      .then(res => res.json())
      .then(data => setClientSecret(data.clientSecret));
  }, [amount]);

  const appearance = {
    theme: 'midnight',      // 'default' | 'midnight' | 'charcoal' | 'brutalist'
    variables: {
      colorPrimary: '#6366F1',
      colorBackground: '#ffffff',
      fontFamily: 'Inter, sans-serif',
      borderRadius: '8px',
    },
  };

  return clientSecret ? (
    <Elements hyper={hyperPromise} options={{ clientSecret, appearance }}>
      <PaymentForm />
    </Elements>
  ) : (
    <div>Loading...</div>
  );
}
```

---

## Step 4: Build the Payment Form

```jsx
// PaymentForm.jsx
import React, { useState } from 'react';
import { useHyper, useElements, PaymentElement } from '@juspay-tech/react-hyper-js';

export default function PaymentForm() {
  const hyper = useHyper();
  const elements = useElements();
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState(null);

  const handleSubmit = async (e) => {
    e.preventDefault();

    if (!hyper || !elements) return;

    setIsProcessing(true);
    setError(null);

    const { error: submitError } = await elements.submit();
    if (submitError) {
      setError(submitError.message);
      setIsProcessing(false);
      return;
    }

    const { error: confirmError } = await hyper.confirmPayment({
      elements,
      confirmParams: {
        return_url: `${window.location.origin}/payment/complete`,
      },
    });

    if (confirmError) {
      setError(confirmError.message);
    }
    // If no error, user is redirected to return_url

    setIsProcessing(false);
  };

  return (
    <form onSubmit={handleSubmit}>
      <PaymentElement
        options={{
          layout: 'tabs',     // 'tabs' | 'accordion' | 'auto'
          wallets: {
            applePay: 'auto',
            googlePay: 'auto',
          },
        }}
      />
      {error && <div className="error">{error}</div>}
      <button type="submit" disabled={isProcessing}>
        {isProcessing ? 'Processing…' : 'Pay Now'}
      </button>
    </form>
  );
}
```

---

## Step 5: Handle the Return URL

After payment (including 3DS redirects), the customer lands on your `return_url`. Verify server-side:

```javascript
// payment/complete.js
import { useEffect, useState } from 'react';
import { useHyper } from '@juspay-tech/react-hyper-js';

export default function PaymentComplete() {
  const hyper = useHyper();
  const [status, setStatus] = useState('loading');

  useEffect(() => {
    const clientSecret = new URLSearchParams(window.location.search).get(
      'payment_intent_client_secret'
    );

    hyper.retrievePaymentIntent(clientSecret).then(({ paymentIntent }) => {
      setStatus(paymentIntent?.status ?? 'unknown');
    });
  }, [hyper]);

  if (status === 'succeeded') return <div>Payment complete! 🎉</div>;
  if (status === 'processing') return <div>Payment processing — we'll notify you.</div>;
  return <div>Payment failed. Please try again.</div>;
}
```

> Always verify the final payment status on your **server** before fulfilling — client-side status can be tampered with.

---

## Appearance Customization

```javascript
const appearance = {
  theme: 'default',   // 'default' | 'midnight' | 'charcoal' | 'brutalist'
  variables: {
    // Colors
    colorPrimary: '#6366F1',
    colorBackground: '#FFFFFF',
    colorText: '#1F2937',
    colorDanger: '#EF4444',
    colorWarning: '#F59E0B',

    // Typography
    fontFamily: '"Inter", "Helvetica Neue", sans-serif',
    fontSizeBase: '16px',
    fontWeightNormal: '400',
    fontWeightMedium: '500',

    // Shape
    borderRadius: '8px',
    spacingUnit: '4px',
  },
  rules: {
    '.Input': {
      border: '1px solid #E5E7EB',
      boxShadow: 'none',
    },
    '.Input:focus': {
      border: '1px solid #6366F1',
      boxShadow: '0 0 0 2px rgba(99, 102, 241, 0.2)',
    },
    '.Label': {
      fontWeight: '500',
      color: '#374151',
    },
  },
};
```

---

## Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `Invalid publishable key` | Wrong `clientKey` passed to `loadHyper` | Use the key from Dashboard → Developers, not the secret API key |
| `client_secret not found` | Payment created without `confirm: false` | Ensure server creates payment with `confirm: false` to get a `client_secret` |
| `Elements not ready` | Calling `confirmPayment` before `elements.submit()` completes | Always `await elements.submit()` before `confirmPayment` |
| Payment redirects loop | `return_url` not handled correctly | Ensure `return_url` page calls `retrievePaymentIntent` and does not re-create payment |

---

## Production Tips

- Never log or persist `client_secret` — it provides write access to the payment object.
- Set `confirm: false` when creating the payment server-side; the SDK handles confirmation client-side.
- Test on real mobile devices — Apple Pay requires HTTPS, a valid Apple Pay domain verification file (`/.well-known/apple-developer-merchantid-domain-association`), and Safari on iOS/macOS.
- For Google Pay, verify your domain is registered in the Google Pay Business Console.
- Use `layout: 'auto'` in `PaymentElement` for optimal rendering across desktop and mobile — it adapts the form layout based on viewport.
