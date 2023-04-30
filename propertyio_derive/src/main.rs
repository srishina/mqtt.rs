use mqttio::errors::Error;
use propertyio_derive::IOOperations;

use mqttio::io::{Reader, Writer};
use mqttio::properties::{PropertyID, PropertyReader, PropertySize, PropertyWriter};
use num::FromPrimitive;

#[derive(Default, IOOperations)]
struct Struct {
    #[ioops(prop_id(PropertyID::WillDelayInterval))]
    d: Option<u32>,
}

fn main() {
    let s: Struct = Default::default();
    println!("Hello, world!!!");
}
