/// IBusText GVariant construction for the IBus D-Bus protocol.
///
/// IBusText is serialized as `(sa{sv}sv)`:
///   "IBusText", {attachments}, "the text", <IBusAttrList variant>
///
/// IBusAttrList is serialized as `(sa{sv}av)`:
///   "IBusAttrList", {attachments}, [attributes]

use std::collections::HashMap;
use zbus::zvariant::{Array, Dict, StructureBuilder, Value};

fn empty_attachments<'a>() -> Value<'a> {
    Value::Dict(
        Dict::try_from(HashMap::<String, Value<'a>>::new())
            .expect("empty a{sv} dict"),
    )
}

fn empty_attr_array<'a>() -> Value<'a> {
    Value::Array(
        Array::try_from(Vec::<Value<'a>>::new()).expect("empty av array"),
    )
}

fn ibus_attr_list<'a>() -> Value<'a> {
    Value::Structure(
        StructureBuilder::new()
            .add_field(Value::from("IBusAttrList".to_string()))
            .add_field(empty_attachments())
            .add_field(empty_attr_array())
            .build(),
    )
}

pub fn ibus_text(s: &str) -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field(Value::from("IBusText".to_string()))
            .add_field(empty_attachments())
            .add_field(Value::from(s.to_string()))
            .add_field(Value::Value(Box::new(ibus_attr_list())))
            .build(),
    )
}
