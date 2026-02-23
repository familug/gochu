/// IBusText GVariant construction for the IBus D-Bus protocol.
///
/// IBusText is serialized as `(sa{sv}sv)`:
///   "IBusText", {attachments}, "the text", <IBusAttrList variant>
///
/// IBusAttrList is serialized as `(sa{sv}av)`:
///   "IBusAttrList", {attachments}, [attributes]
///
/// When building a Structure with StructureBuilder, each field's D-Bus type is
/// determined by `DynamicType::dynamic_signature()`. For `Value`, this always
/// returns "v" (because `Value` IS a D-Bus variant), which produces the wrong
/// struct signature `(vvvv)` instead of `(sa{sv}sv)`.
///
/// We use `Field` — a thin wrapper that reports the *contained* type's
/// signature via `Value::value_signature()` — so that strings contribute "s",
/// dicts contribute "a{sv}", etc.

use std::collections::HashMap;
use zbus::zvariant::{Array, Dict, DynamicType, Signature, StructureBuilder, Value};

/// Wrapper that reports the enclosed value's actual type signature to
/// StructureBuilder, rather than the generic variant signature "v".
struct Field<'a> {
    value: Value<'a>,
    sig: Signature<'static>,
}

impl<'a> Field<'a> {
    /// Use the value's own type signature.
    fn auto(value: Value<'a>) -> Self {
        let sig = value.value_signature().to_owned();
        Self { value, sig }
    }

    /// Override the signature to "v" (D-Bus variant). Use this when the struct
    /// field must be typed as variant but the stored Value is already the
    /// concrete inner value (avoiding double variant wrapping).
    fn as_variant(value: Value<'a>) -> Self {
        Self {
            value,
            sig: Signature::from_static_str_unchecked("v"),
        }
    }
}

impl DynamicType for Field<'_> {
    fn dynamic_signature(&self) -> Signature<'_> {
        self.sig.as_ref()
    }
}

impl<'a> From<Field<'a>> for Value<'a> {
    fn from(f: Field<'a>) -> Self {
        f.value
    }
}

fn empty_attachments<'a>() -> Value<'a> {
    Value::Dict(
        Dict::try_from(HashMap::<String, Value<'a>>::new()).expect("empty a{sv} dict"),
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
            .add_field(Field::auto(Value::from("IBusAttrList".to_string())))
            .add_field(Field::auto(empty_attachments()))
            .add_field(Field::auto(empty_attr_array()))
            .build(),
    )
}

pub fn ibus_text(s: &str) -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field(Field::auto(Value::from("IBusText".to_string())))
            .add_field(Field::auto(empty_attachments()))
            .add_field(Field::auto(Value::from(s.to_string())))
            .add_field(Field::as_variant(ibus_attr_list()))
            .build(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::{serialized, LE};

    #[test]
    fn ibus_text_inner_signature_is_correct() {
        let text = ibus_text("hello");
        assert_eq!(text.dynamic_signature().as_str(), "v");

        match &text {
            Value::Structure(s) => {
                assert_eq!(s.signature().as_str(), "(sa{sv}sv)");
            }
            _ => panic!("expected Structure"),
        }
    }

    #[test]
    fn ibus_attr_list_inner_signature_is_correct() {
        let attr = ibus_attr_list();
        match &attr {
            Value::Structure(s) => {
                assert_eq!(s.signature().as_str(), "(sa{sv}av)");
            }
            _ => panic!("expected Structure"),
        }
    }

    #[test]
    fn commit_text_body_signature_is_v() {
        assert_eq!(ibus_text("hello").dynamic_signature().as_str(), "v");
    }

    #[test]
    fn update_preedit_body_signature() {
        let body = (ibus_text("test"), 4u32, true);
        assert_eq!(body.dynamic_signature().as_str(), "(vub)");
    }

    #[test]
    fn hide_preedit_body_signature_is_empty() {
        assert_eq!(().dynamic_signature().as_str(), "");
    }

    #[test]
    fn commit_text_serialized_variant_signature() {
        let body = ibus_text("hello");
        let ctxt = serialized::Context::new_dbus(LE, 0);
        let encoded = zbus::zvariant::to_bytes(ctxt, &body).expect("serialize");
        let bytes = encoded.bytes();

        let sig_len = bytes[0] as usize;
        let sig = std::str::from_utf8(&bytes[1..1 + sig_len]).expect("utf8");
        assert_eq!(sig, "(sa{sv}sv)");
    }

    /// Roundtrip: serialize → deserialize, verify that deserialized fields have
    /// the correct D-Bus types (no extra variant wrapping).
    #[test]
    fn commit_text_roundtrip_field_types() {
        let body = ibus_text("xin chào");
        let ctxt = serialized::Context::new_dbus(LE, 0);
        let encoded = zbus::zvariant::to_bytes(ctxt, &body).expect("serialize");
        let (decoded, _): (Value, _) = encoded.deserialize().expect("deserialize");

        match decoded {
            Value::Structure(s) => {
                assert_eq!(s.signature().as_str(), "(sa{sv}sv)");
                let fields = s.fields();
                assert_eq!(fields.len(), 4);

                // Field 0: string "IBusText" (type s, not v)
                assert!(
                    matches!(&fields[0], Value::Str(s) if s.as_str() == "IBusText"),
                    "field 0: expected Str(\"IBusText\"), got {:?}",
                    fields[0]
                );

                // Field 1: empty dict (type a{sv})
                assert!(
                    matches!(&fields[1], Value::Dict(_)),
                    "field 1: expected Dict, got {:?}",
                    fields[1]
                );

                // Field 2: the text (type s)
                assert!(
                    matches!(&fields[2], Value::Str(s) if s.as_str() == "xin chào"),
                    "field 2: expected Str(\"xin chào\"), got {:?}",
                    fields[2]
                );

                // Field 3: variant containing IBusAttrList (type v).
                // Deserialization wraps variant content in Value::Value, so
                // we expect Value::Value(Structure) — ONE layer, not two.
                match &fields[3] {
                    Value::Value(inner) => match inner.as_ref() {
                        Value::Structure(attr_s) => {
                            assert_eq!(attr_s.signature().as_str(), "(sa{sv}av)");
                        }
                        other => panic!(
                            "field 3 inner: expected Structure (1 variant layer), got {other:?}"
                        ),
                    },
                    other => panic!("field 3: expected Value (variant), got {other:?}"),
                }
            }
            other => panic!("expected Structure, got {other:?}"),
        }
    }
}
