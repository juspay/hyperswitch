use masking::Secret;
use serde::Serialize;
use masking::MaskedSerialize;
pub use erased_serde::Serialize as ErasedSerialize;

#[derive(Serialize, Debug, Clone)]
pub struct Test {
    pub name: Secret<String>,
    pub age: u8,
}

fn main() {
    let person = Test {
        name: "Soppa".to_owned().into(),
        age: 23,
    };
    println!("hello world, {:?}", person);
    let masked = masking::masked_serialize(&person).unwrap();
    let erased_masked = mask_stuff(Box::new(person));
    // let serialized = serde_json::to_string(&masked).unwrap();
    println!(
        "hello world, {:?}\n erased_masked:\n{:?}",
        masked, erased_masked
    );
}

fn mask_stuff(body: Box<dyn MaskedSerialize>) -> serde_json::Value {
    // let masked = masking::masked_serialize(&body).unwrap();
    serde_json::to_value(&body).unwrap()
}
