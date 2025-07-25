use std::fmt::Debug;

#[allow(dead_code)]
pub trait PIIHolder {
    type CardNum: Default + Debug;
    type CardCvc: Default + Debug;
    type PinCode: Default + Debug;
    type AddressGeneric: Default + Debug;
}

pub trait PIIInner {
    type Inner: Default + Debug;
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Card<T: PIIHolder> {
    pub card_number: T::CardNum,
    pub card_cvc: T::CardCvc,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Address<T: PIIHolder> {
    pub state: T::AddressGeneric,
    pub city: T::AddressGeneric,
    pub pincode: T::PinCode,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct PaymentDetails<T: PIIHolder + PIIInner> {
    pub address: Address<T>,
    pub card: Card<T>,
}

#[derive(Default, Debug)]
pub struct DefaultPIIHolder;

impl PIIHolder for DefaultPIIHolder {
    type CardNum = String;
    type CardCvc = u8;
    type PinCode = String;
    type AddressGeneric = String;
}

impl PIIInner for DefaultPIIHolder {
    type Inner = String;
}

#[derive(Default, Debug)]
pub struct FunkyPIIHolder;

impl PIIHolder for FunkyPIIHolder {
    type CardNum = u8;
    type CardCvc = bool;
    type PinCode = Vec<String>;
    type AddressGeneric = Option<String>;
}

impl PIIInner for FunkyPIIHolder {
    type Inner = u8;
}

// Previously defined: CardProxy, Flow, RouterData
#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct CardProxy<T: PIIHolder> {
    pub proxy_card_id: Option<T::CardNum>,
    pub proxy_cvc_list: Vec<T::CardCvc>,
}

#[derive(Debug)]
pub enum Flow {
    Normal,
    Proxy,
}

#[derive(Debug)]
pub enum RouterData<T: PIIHolder> {
    NormalCard(Card<T>),
    ProxyCard(CardProxy<T>),
}

// Task implementations start here

// 1. Define placeholder functions
pub fn fx() {
    println!("fx() function called");
}

pub fn fy() {
    println!("fy() function called");
}

pub fn fa() {
    println!("fa() function called");
}

pub fn fb() {
    println!("fb() function called");
}

// 2. Define ConnectorRequest<T> struct
#[derive(Debug)]
pub struct ConnectorRequest<T: PIIHolder> {
    pub router_data: RouterData<T>,
}

// 3. Implement call_connector_service function
pub fn call_connector_service<T: PIIHolder + Default>(flow: Flow) -> RouterData<T> {
    match flow {
        Flow::Normal => RouterData::NormalCard(Card::default()),
        Flow::Proxy => RouterData::ProxyCard(CardProxy::default()),
    }
}

// 4. Implement f4 function
pub fn f4<T: PIIHolder>(router_data: RouterData<T>) -> ConnectorRequest<T> {
    ConnectorRequest { router_data }
}

// 5. Implement f1 function (Normal Card Flow)
pub fn f1<T: PIIHolder + Default>() -> ConnectorRequest<T> {
    fx();
    fy();
    let router_data = call_connector_service(Flow::Normal);
    f4(router_data)
}

// 6. Implement f2 function (Proxy Flow)
pub fn f2<T: PIIHolder + Default>() -> ConnectorRequest<T> {
    fa();
    fb();
    let router_data = call_connector_service(Flow::Proxy);
    f4(router_data)
}

// 7. Main function demonstrating usage
fn main() {
    println!("=== Demonstrating f1 (Normal Card Flow) with DefaultPIIHolder ===");
    let default_normal_request: ConnectorRequest<DefaultPIIHolder> = f1();
    println!("f1() with DefaultPIIHolder result: {:?}", default_normal_request);

    println!("\n=== Demonstrating f2 (Proxy Flow) with DefaultPIIHolder ===");
    let default_proxy_request: ConnectorRequest<DefaultPIIHolder> = f2();
    println!("f2() with DefaultPIIHolder result: {:?}", default_proxy_request);

    println!("\n=== Demonstrating f1 (Normal Card Flow) with FunkyPIIHolder ===");
    let funky_normal_request: ConnectorRequest<FunkyPIIHolder> = f1();
    println!("f1() with FunkyPIIHolder result: {:?}", funky_normal_request);

    println!("\n=== Demonstrating f2 (Proxy Flow) with FunkyPIIHolder ===");
    let funky_proxy_request: ConnectorRequest<FunkyPIIHolder> = f2();
    println!("f2() with FunkyPIIHolder result: {:?}", funky_proxy_request);

    println!("\n=== Additional demonstrations ===");
    
    // Show the flow of function calls more explicitly
    println!("\nExplicit flow demonstration:");
    println!("1. Calling f1<DefaultPIIHolder>():");
    println!("   - This will call fx() and fy()");
    println!("   - Then call call_connector_service(Flow::Normal)");
    println!("   - Finally call f4() to wrap in ConnectorRequest");
    
    println!("\n2. Calling f2<FunkyPIIHolder>():");
    println!("   - This will call fa() and fb()");
    println!("   - Then call call_connector_service(Flow::Proxy)");
    println!("   - Finally call f4() to wrap in ConnectorRequest");
}
