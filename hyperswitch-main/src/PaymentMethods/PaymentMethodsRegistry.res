// PaymentMethodsRegistry.res
// Registry for payment method components

open MandiriVaBankTransfer
open GivexGiftCard
open DynamicFieldTypes

type paymentMethodType =
  | MandiriVa
  | Givex
  | Other(string)

type paymentMethodComponent =
  | SimpleComponent((~onPaymentSuccess: Js.Json.t => unit, ~onPaymentError: string => unit) => React.element)
  | DynamicComponent((~config: dynamicFieldConfig, ~onPaymentSuccess: Js.Json.t => unit, ~onPaymentError: string => unit) => React.element)

type paymentMethodInfo = {
  component: paymentMethodComponent,
  displayName: string,
  description: string,
}

let getPaymentMethodInfo = (paymentMethod: paymentMethodType): paymentMethodInfo => {
  switch paymentMethod {
  | MandiriVa => {
      component: SimpleComponent(MandiriVaBankTransfer.make),
      displayName: "Mandiri VA Bank Transfer",
      description: "Pay using Mandiri Virtual Account",
    }
  | Givex => {
      component: DynamicComponent(GivexGiftCard.make),
      displayName: "Givex Gift Card",
      description: "Pay using Givex Gift Card",
    }
  | Other(name) => {
      component: SimpleComponent((~onPaymentSuccess, ~onPaymentError) => {
        <div>
          {React.string(`Payment method "${name}" is not implemented yet`)}
        </div>
      }),
      displayName: name,
      description: "Payment method not available",
    }
  }
}

let getAvailablePaymentMethods = (): array<paymentMethodType> => {
  [MandiriVa, Givex]
}

let stringToPaymentMethodType = (str: string): paymentMethodType => {
  switch str {
  | "mandiri_va" => MandiriVa
  | "givex" => Givex
  | other => Other(other)
  }
}