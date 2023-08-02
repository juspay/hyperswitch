pub const PAYMENT_ID_PREFIX: &str = "dummy_pay";
pub const ATTEMPT_ID_PREFIX: &str = "dummy_attempt";
pub const REFUND_ID_PREFIX: &str = "dummy_ref";
pub const THREE_DS_CSS: &str = r#"
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@200;300;400;500;600;700&display=swap');
    body {
        font-family: Inter;
        background-image: url('https://app.hyperswitch.io/images/hyperswitchImages/PostLoginBackground.svg');
        display: flex; 
        background-size: cover;
        height: 100%; 
        padding-top: 3rem;
        margin: 0; 
        align-items: center; 
        flex-direction: column; 
    }
    .authorize {
        color: white;
        background-color: #006DF9;
        border: 1px solid #006DF9;
        box-sizing: border-box;
        margin: 1.5rem 0.5rem 1rem 0;
        border-radius: 0.25rem;
        padding: 0.75rem 1rem;
        font-weight: 500;
        font-size: 1.1rem;
    }
    .authorize:hover {
        background-color: #0099FF;
        border: 1px solid #0099FF;
        cursor: pointer;
    }
    .reject {
        background-color: #F7F7F7;
        color: black;
        border: 1px solid #E8E8E8;
        box-sizing: border-box;
        border-radius: 0.25rem;
        margin-left: 0.5rem;
        font-size: 1.1rem;
        padding: 0.75rem 1rem;
        font-weight: 500;
    }
    .reject:hover {
        cursor: pointer;
        background-color: #E8E8E8;
        border: 1px solid #E8E8E8;
    }
    .container {
        background-color: white;
        width: 33rem;
        margin: 1rem 0;
        border: 1px solid #E8E8E8;
        color: black;
        border-radius: 0.25rem;
        padding: 1rem 1.4rem 1rem 1.4rem;
        font-family: Inter;
    }
    .container p {
        font-weight: 400;
        margin-top: 0.5rem 1rem 0.5rem 0.5rem;
        color: #151A1F;
        opacity: 0.5;
    }
    b {
        font-weight: 600;
    }
    .disclaimer {
        font-size: 1.25rem;
        font-weight: 500 !important;
        margin-top: 0.5rem !important;
        margin-bottom: 0.5rem;
        opacity: 1 !important;
    }
    .heading {
        display: flex;
        justify-content: center;
        flex-direction: column;
        align-items: center;
        margin-bottom: 1rem;
    }
    .logo {
        width: 8rem;
    }
    .payment_details {
        height: 2rem;
        display: flex;
        margin: 1rem 0 2rem 0;
    }
    .border_horizontal {
        border-top: 1px dashed #151a1f80;
        height: 1px;
        margin: 0 1rem;
        margin-top: 1rem;
        width: 20%;
    }
    .contact {
        display: flex;
        gap: 10%;
        margin: 2rem 0 1rem 0;
        height: 3rem;
    }
    .contact img {
        aspect-ratio: 1/1;
        height: 2rem;
    }
    .contact_item {
        display: flex;
        height: 100%;
        flex-direction: column;
        justify-content: center;
    }
    .contact_item p {
        margin: 0;
    }
    .border_vertical {
        border-left: 1px solid #151a1f80;
        width: 1px;
        height: 100%;
    }
    .email {
        justify-content: space-between;
    }
    .hover_cursor:hover {
        cursor: pointer
    }
    a {
        text-decoration: none;
        opacity: 0.8;
    }"#;
