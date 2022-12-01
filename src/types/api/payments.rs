#[derive(Default, Debug)]
pub struct PaymentsRequest{
  pub payment_id : Option<String>,
  pub amount : f32,
  pub currency : String,
  pub capture_method : Option<CaptureMethod>,
  pub amount_to_capture : Option<f32>,
  pub capture_on : Option<String>, //is thercleare a better way to define date and time?
  pub confirm : Option<bool>,
  pub customer : Option<String>,
  pub customer_email : Option<String>,
  pub customer_name : Option<String>,
  pub description : Option<String>,
  pub return_url : Option<String>,
  pub setup_future_usage : Option<FutureUsage>,
  pub payment_method_data : Option<PaymentMethod>,
  pub payment_method : Option<String>,
  pub shipping : Option<Address> ,
  pub billing : Option<Address>,
  pub statement_descriptor_name : Option<String>,
  pub statement_descriptor_suffix : Option<String>,
}

  #[derive(Default, Debug)]
          pub struct CCard {
            pub card_number : String,
            pub card_exp_month : String,
            pub card_exp_year : String,
            pub card_holder_name : String,
            pub card_cvc : String,
            }

          #[derive(Debug)]  
          pub enum PaymentMethod {
            Card(CCard),
            BankTranfer,
            }

          impl Default for PaymentMethod {
              fn default() -> Self { PaymentMethod::BankTranfer }
          }   

          #[derive(Debug)]
          pub enum CaptureMethod {
            Automatic,
            Manual,
          }

          impl Default for CaptureMethod {
            fn default() -> Self {CaptureMethod::Manual}
          }
          
          #[derive(Debug)]
          pub enum FutureUsage {
            Required,
            Optional,
          }

          impl Default for FutureUsage {
            fn default() -> Self {FutureUsage::Optional}
          }

          #[derive(Default, Debug)]
          pub struct Address {
            pub address : Option<AddressDetails>,
            pub phone : Option<PhoneDetails>,
            }
            
            #[derive(Default, Debug)]
            pub struct AddressDetails {
                    pub city : Option<String>,
                    pub country : Option<String>,
                    pub line1 : Option<String>,
                    pub line2 : Option<String>,
                    pub line3 : Option<String>,
                    pub zip : Option<String>,
                    pub state : Option<String>,
                    pub first_name : Option<String>,
                    pub last_name : Option<String>,
                    }
                  
                    #[derive(Default, Debug)]
                    pub struct PhoneDetails {
                      pub number : Option<String>,
                      pub country_code : Option<String>,
                    }

#[derive(Default, Debug)]
pub struct PaymentsResponse{        
  pub payment_id: String,
  pub status : PaymentStatus,
  pub amount: f32,
  pub amount_capturable: f32,
  pub amount_received: f32,
  pub client_secret: String,
  pub created: String,
  pub currency: String,
  pub customer: Option<String>,
  pub description: Option<String>,
}  

#[derive(Debug)]
pub enum PaymentStatus {
#[derive(Debug)]
pub enum PaymentStatus {
  Suceeded,
  Failed,
  Processing,
  RequiresCustomerAction,
  RequiresPaymentMethod,
  RequiresConfirmation,
}

impl Default for PaymentStatus {
  fn default() -> Self {PaymentStatus::Failed}
}

#[cfg(test)]
mod payments_test {
  use super::*;

  #[test]
  fn verify_payments_request () {
    let pay_req = PaymentsRequest {
      amount:200.0,
      ..Default::default()
    };
    println!("{:#?}", pay_req);
    assert_eq!(true, true);
  }

  #[test]
  fn verify_payments_response () {
    let pay_res = PaymentsResponse {
      amount: 2000.0,
      ..Default::default()
    };
    println!("{:#?}", pay_res);
    assert_eq!(true, true);
  }
}
