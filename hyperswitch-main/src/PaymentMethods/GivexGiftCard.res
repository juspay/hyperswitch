// GivexGiftCard.res
// Givex Gift Card payment method component

open DynamicFieldTypes
open DynamicFieldsContainer

@react.component
let make = (~config: dynamicFieldConfig, ~onPaymentSuccess: Js.Json.t => unit, ~onPaymentError: string => unit) => {
  let (fieldValues, setFieldValues) = React.useState(() => Js.Dict.empty())

  let handleFieldChange = (fieldKey: string, value: string) => {
    let newFieldValues = Js.Dict.copy(fieldValues)
    Js.Dict.set(newFieldValues, fieldKey, value)
    setFieldValues(_ => newFieldValues)
  }

  let handleSubmit = (_: ReactEvent.Form.t) => {
    // Validate all required fields
    let requiredFields = Js.Dict.entries(config.requiredFields)
    let validationErrors = []

    requiredFields->Belt.Array.forEach(((fieldKey, field)) => {
      let value = Js.Dict.get(fieldValues, fieldKey)->Belt.Option.getWithDefault("")
      let validation = DynamicFieldValidation.validateFieldValue(field, value)
      if !validation.isValid {
        validationErrors->Belt.Array.push((field.displayName, validation.errorMessage))
      }
    })

    if Belt.Array.length(validationErrors) > 0 {
      let errorMessage = validationErrors
        ->Belt.Array.map(((fieldName, error)) => `${fieldName}: ${error->Belt.Option.getWithDefault("Invalid")}`)
        ->Belt.Array.joinWith(", ")
      onPaymentError(`Validation errors: ${errorMessage}`)
    } else {
      // Construct payment data according to the required payload structure
      let giftCardData = Js.Dict.empty()

      // Extract values for gift_card fields
      let number = Js.Dict.get(fieldValues, "payment_method_data.gift_card.number")->Belt.Option.getWithDefault("")
      let cvc = Js.Dict.get(fieldValues, "payment_method_data.gift_card.cvc")->Belt.Option.getWithDefault("")

      Js.Dict.set(giftCardData, "number", Js.Json.string(number))
      Js.Dict.set(giftCardData, "cvc", Js.Json.string(cvc))

      let givexData = Js.Dict.empty()
      Js.Dict.set(givexData, "givex", Js.Json.object_(giftCardData))

      let paymentMethodData = Js.Dict.empty()
      Js.Dict.set(paymentMethodData, "gift_card", Js.Json.object_(givexData))

      let paymentData = Js.Dict.empty()
      Js.Dict.set(paymentData, "payment_method", Js.Json.string(config.paymentMethod))
      Js.Dict.set(paymentData, "payment_method_type", Js.Json.string(config.paymentMethodType))
      Js.Dict.set(paymentData, "payment_method_data", Js.Json.object_(paymentMethodData))

      // Mock API call - replace with actual API integration
      Js.log("Submitting Givex payment with data:")
      Js.log(paymentData)

      // Simulate API response
      let response = Js.Json.object_(paymentData)
      onPaymentSuccess(response)
    }
  }

  <div className="givex-payment-container">
    <h3 className="payment-method-title"> {React.string("Givex Gift Card")} </h3>
    <p className="payment-method-description">
      {React.string("Complete your payment using Givex Gift Card. Enter your card details below.")}
    </p>

    <form className="dynamic-fields-form" onSubmit=handleSubmit>
      {Js.Dict.entries(config.requiredFields)
      ->Belt.Array.map(((fieldKey, field)) => {
        let currentValue = Js.Dict.get(fieldValues, fieldKey)->Belt.Option.getWithDefault("")

        <DynamicField
          key=fieldKey
          field=field
          value=currentValue
          onChange={value => handleFieldChange(fieldKey, value)}
          onBlur={() => ()}
        />
      })
      ->React.array}

      <button type_="submit" className="dynamic-fields-submit-btn">
        {React.string("Pay with Givex")}
      </button>
    </form>
  </div>
}