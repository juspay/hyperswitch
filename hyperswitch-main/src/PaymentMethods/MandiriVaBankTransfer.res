// MandiriVaBankTransfer.res
// Mandiri VA Bank Transfer payment method component

open DynamicFieldTypes
open DynamicFieldsContainer

@react.component
let make = (~onPaymentSuccess: Js.Json.t => unit, ~onPaymentError: string => unit) => {
  let (fieldValues, setFieldValues) = React.useState(() => Js.Dict.empty())

  // Configuration for Mandiri VA fields
  let mandiriVaConfig: dynamicFieldConfig = {
    fields: [
      {
        fieldType: UserFullName,
        label: "Full Name",
        placeholder: "Enter your full name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(100),
        minLength: Some(2),
      },
      {
        fieldType: UserEmailAddress,
        label: "Email Address",
        placeholder: "Enter your email address",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(254),
        minLength: None,
      },
      {
        fieldType: UserPhoneNumber,
        label: "Phone Number",
        placeholder: "Enter your phone number",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(15),
        minLength: Some(10),
      },
      {
        fieldType: UserMandiriVaNumber,
        label: "Mandiri VA Number",
        placeholder: "Enter your Mandiri VA number",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(16),
        minLength: Some(10),
      },
      {
        fieldType: UserPaymentAmount,
        label: "Payment Amount",
        placeholder: "Enter payment amount",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      },
      {
        fieldType: UserPaymentCurrency,
        label: "Currency",
        placeholder: "IDR",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(3),
        minLength: Some(3),
      },
      {
        fieldType: UserPaymentDescription,
        label: "Payment Description",
        placeholder: "Enter payment description",
        required: false,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(255),
        minLength: None,
      },
    ],
    submitButtonText: "Pay with Mandiri VA",
    onSubmit: (fieldValuePairs) => {
      // Construct payment data
      let paymentData = Js.Dict.empty()

      fieldValuePairs->Belt.Array.forEach(((fieldTypeStr, value)) => {
        Js.Dict.set(paymentData, fieldTypeStr, Js.Json.string(value))
      })

      // Add payment method specific data
      Js.Dict.set(paymentData, "payment_method", Js.Json.string("bank_transfer"))
      Js.Dict.set(paymentData, "payment_method_type", Js.Json.string("mandiri_va"))

      // Mock API call - replace with actual API integration
      Js.log("Submitting Mandiri VA payment with data:")
      Js.log(paymentData)

      // Simulate API response
      let response = Js.Json.object_(paymentData)
      onPaymentSuccess(response)
    },
  }

  let handleFieldChange = (fieldType: fieldType, value: string) => {
    let newFieldValues = Js.Dict.copy(fieldValues)
    Js.Dict.set(newFieldValues, fieldTypeToString(fieldType), value)
    setFieldValues(_ => newFieldValues)
  }

  <div className="mandiri-va-payment-container">
    <h3 className="payment-method-title"> {React.string("Mandiri VA Bank Transfer")} </h3>
    <p className="payment-method-description">
      {React.string("Complete your payment using Mandiri Virtual Account. You'll receive a VA number to make the transfer.")}
    </p>

    <DynamicFieldsContainer
      config=mandiriVaConfig
      fieldValues=fieldValues
      onFieldChange=handleFieldChange
    />
  </div>
}