# Try out hyperswitch sandbox environment

**Table Of Contents:**

- [Set up your accounts](#set-up-your-accounts)
- [Try out our APIs](#try-out-our-apis)
  - [Create a payment](#create-a-payment)
  - [Create a refund](#create-a-refund)

## Set up your accounts

1. Sign up on the payment connector's (say Stripe, Adyen, etc.) dashboard and
   store your connector API key (and any other necessary secrets) securely.
2. Sign up on our [dashboard][dashboard].
3. Create a merchant account on our dashboard and generate your API keys.
   Ensure to save the merchant ID, API key and publishable key displayed on the
   dashboard securely.
4. Configure the merchant return URL and the webhooks URL, which will be used
   on completion of payments and for sending webhooks, respectively.
5. Create a payments connector account by selecting a payment connector among
   the options displayed and fill in the connector credentials you obtained in
   Step 1.
6. Sign up or sign in to [Postman][postman].
7. Open our [Postman collection][postman-collection] and switch to the
   ["Variables" tab][variables].
   Add the API key received in Step 3 under the "current value" column for the
   `api_key` variable.

## Try out our APIs

### Create a payment

1. Open the ["Quick Start" folder][quick-start] in the collection.
2. Open the ["Payments - Create"][payments-create] request, switch to the "Body"
   tab and update any request parameters as required.
   Click on the "Send" button to create a payment.
   If all goes well and you had provided the correct connector credentials, the
   payment should be created successfully.
   You should see the `status` field of the response body having a value of
   `succeeded` in this case.

   - If the `status` of the payment created was `requires_confirmation`, set
     `confirm` to `true` in the request body and send the request again.

3. Open the ["Payments - Retrieve"][payments-retrieve] request and click on the
   "Send" button (without modifying anything).
   This should return the payment object for the payment created in Step 2.

### Create a refund

1. Open the ["Refunds - Create"][refunds-create] request in the
   ["Quick Start" folder][quick-start] folder and switch to the "Body" tab.
   Update the amount to be refunded, if required, and click on the "Send" button.
   This should create a refund against the last payment made for the specified
   amount.
   Check the `status` field of the response body to verify that the refund
   hasn't failed.
2. Open the ["Refunds - Retrieve"][refunds-retrieve] request and switch to the
   "Params" tab.
   Set the `id` path variable in the "Path Variables" table to the `refund_id`
   value returned in the response during the previous step.
   This should return the refund object for the refund created in the previous
   step.

That's it!
Hope you got a hang of our APIs.
To explore more of our APIs, please check the remaining folders in the
[Postman collection][postman-collection].

[dashboard]: https://app.hyperswitch.io
[postman]: https://www.postman.com
[postman-collection]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed
[variables]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed?tab=variables
[quick-start]: https://www.postman.com/hyperswitch/workspace/hyperswitch/folder/25176183-0103918c-6611-459b-9faf-354dee8e4437
[payments-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-9b4ad6a8-fbdd-4919-8505-c75c83bdf9d6
[payments-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-11995c9b-8a34-4afd-a6ce-e8645693929b
[refunds-create]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-5b15d068-db9e-48a5-9ee9-3a70c0aac944
[refunds-retrieve]: https://www.postman.com/hyperswitch/workspace/hyperswitch/request/25176183-c50c32af-5ceb-4ab6-aca7-85f6b32df9d3
