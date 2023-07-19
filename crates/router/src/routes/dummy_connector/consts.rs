pub const PAYMENT_ID_PREFIX: &str = "dummy_pay";
pub const ATTEMPT_ID_PREFIX: &str = "dummy_attempt";
pub const THREE_DS_CSS: &str = "body {
        background-color: #242F48; 
        display: flex; 
        justify-content: center; 
        height: 100%; 
        margin: 0; 
        align-items: center; 
        flex-direction: column; 
    } 
    img { 
        width: 250px;
        margin-bottom: 2.5rem;
    }
    .authorize {
        color: white;
        background-color: #006DF9;
        border-radius: 10rem;
        border: none;
        padding: 0.75rem 1.5rem;
        margin-top: 2rem;
        font-size: 1.25rem;
        font-weight: 500;
    }
    .authorize:hover {
        background-color: #0099FF;
        cursor: pointer;
    }
    .reject {
        background-color: transparent;
        color: white;
        border-radius: 10rem;
        border: none;
        padding: 0.75rem 1.5rem;
        font-size: 1.25rem;
        font-weight: 500;
    }
    .reject:hover {
        cursor: pointer;
        color: #0099FF;
    }
    p {
        width: 500px;
        color: white;
        margin: 0.5rem;
        font-family: arial;
        text-align: center;
        font-size: 1.25rem;
    }";
