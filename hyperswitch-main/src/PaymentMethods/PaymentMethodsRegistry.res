// PaymentMethodsRegistry.res
// Registry for payment method components

open MandiriVaBankTransfer

type paymentMethodType =
  | MandiriVa
  | Other(string)

type paymentMethodComponent = {
  component: (~onPaymentSuccess: Js.Json.t => unit, ~onPaymentError: string => unit) => React.element,
  displayName: string,
  description: string,
}

let getPaymentMethodComponent = (paymentMethod: paymentMethodType): paymentMethodComponent => {
  switch paymentMethod {
  | MandiriVa => {
      component: MandiriVaBankTransfer.make,
      displayName: "Mandiri VA Bank Transfer",
      description: "Pay using Mandiri Virtual Account",
    }
  | Other(name) => {
      component: (~onPaymentSuccess, ~onPaymentError) => {
        <div>
          {React.string(`Payment method "${name}" is not implemented yet`)}
        </div>
      },
      displayName: name,
      description: "Payment method not available",
    }
  }
}

let getAvailablePaymentMethods = (): array<paymentMethodType> => {
  [MandiriVa]
}

let stringToPaymentMethodType = (str: string): paymentMethodType => {
  switch str {
  | "mandiri_va" => MandiriVa
  | other => Other(other)
  }
}