pub struct RefundRequest{
    pub refund_id : String,
    pub amount : Option<f32>,
    pub reason : Option<String>,
  }

pub struct RefundResponse{
    pub id: String,
    pub amount: f32,
    pub currency: String,
    pub reason: Option<String>,
    pub status: RefundStatus,
    }
    
            #[derive(Debug)]  
            pub enum RefundStatus {
              Succeeded,
              Failed,
              Pending,
              Review,
              }
  