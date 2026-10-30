//! Strict, bounded public value decoding and TI-044 Appendix C framing.
use crate::*;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use std::fmt;

struct UniqueJson(serde_json::Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(n.into()))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut out = vec![];
                while let Some(v) = seq.next_element::<UniqueJson>()? {
                    out.push(v.0);
                }
                Ok(UniqueJson(out.into()))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, v)) = map.next_entry::<String, UniqueJson>()? {
                    if out.insert(k, v.0).is_some() {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(UniqueJson(out.into()))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<serde_json::Value> {
    if bytes.len() > 131072 {
        return err("TI_LIMIT", "encoded inputs exceed 128 KiB");
    }
    serde_json::from_slice::<UniqueJson>(bytes)
        .map(|v| v.0)
        .map_err(|_| Diagnostic {
            code: "TI_INPUT".into(),
            message: "invalid JSON or duplicate key".into(),
            line: 0,
            column: 0,
        })
}
fn decode(v: &serde_json::Value) -> Result<Value> {
    let o = v.as_object().ok_or_else(|| Diagnostic {
        code: "TI_INPUT".into(),
        message: "tagged value required".into(),
        line: 0,
        column: 0,
    })?;
    let tag = o.get("type").and_then(|x| x.as_str()).unwrap_or("");
    if tag == "null" {
        if o.len() != 1 {
            return err("TI_INPUT", "null must not contain value");
        };
        return Ok(Value::Null);
    }
    if o.len() != 2 || !o.contains_key("value") {
        return err("TI_INPUT", "unknown or missing value members");
    }
    let x = &o["value"];
    let string = || {
        x.as_str().ok_or_else(|| Diagnostic {
            code: "TI_INPUT".into(),
            message: "string value required".into(),
            line: 0,
            column: 0,
        })
    };
    match tag {
        "Bool" => x.as_bool().map(Value::Bool).ok_or_else(|| Diagnostic {
            code: "TI_INPUT".into(),
            message: "Boolean required".into(),
            line: 0,
            column: 0,
        }),
        "String" => {
            let s = string()?;
            if s.len() > 16384 {
                return err("TI_LIMIT", "string too long");
            };
            Ok(Value::String(s.into()))
        }
        "Int64" => {
            let s = string()?;
            if !s.parse::<i64>().is_ok_and(|n| n.to_string() == s) {
                return err("TI_INPUT", "canonical Int64 string required");
            };
            Ok(Value::Int64(s.into()))
        }
        "UserId" => {
            let s = string()?;
            id_check(s)?;
            Ok(Value::UserId(s.into()))
        }
        _ if tag.ends_with("Id") => {
            let entity = &tag[..tag.len() - 2];
            if entity.is_empty()
                || entity.len() > 63
                || !entity.bytes().enumerate().all(|(i, c)| {
                    c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit())
                })
            {
                return err("TI_INPUT", "invalid nominal ID tag");
            };
            let s = string()?;
            id_check(s)?;
            Ok(Value::Id(IdValue {
                entity: entity.into(),
                id: s.into(),
            }))
        }
        _ => err("TI_INPUT", "unknown value tag"),
    }
}
fn id_check(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 256 {
        return err("TI_INPUT", "invalid opaque ID");
    };
    Ok(())
}
pub fn decode_inputs(bytes: &[u8]) -> Result<Map<String, Value>> {
    let json = strict_json(bytes)?;
    let object = json.as_object().ok_or_else(|| Diagnostic {
        code: "TI_INPUT".into(),
        message: "input object required".into(),
        line: 0,
        column: 0,
    })?;
    if object.len() > 64 {
        return err("TI_LIMIT", "too many inputs");
    }
    object
        .iter()
        .map(|(n, v)| Ok((n.clone(), decode(v)?)))
        .collect()
}
pub fn encode_value(v: &Value) -> Result<serde_json::Value> {
    use serde_json::json;
    Ok(match v {
        Value::Null => json!({"type":"null"}),
        Value::Id(id) => {
            id_check(&id.id)?;
            json!({"type":format!("{}Id",id.entity),"value":id.id})
        }
        Value::State(_) => return err("TI_TYPE", "state is not a public scalar"),
        _ => serde_json::to_value(v).map_err(|_| Diagnostic {
            code: "TI_INPUT".into(),
            message: "invalid value".into(),
            line: 0,
            column: 0,
        })?,
    })
}
pub fn frame(tag: u8, bytes: &[u8]) -> Result<Vec<u8>> {
    let n = u32::try_from(bytes.len()).map_err(|_| Diagnostic {
        code: "TI_LIMIT".into(),
        message: "frame overflow".into(),
        line: 0,
        column: 0,
    })?;
    let mut out = vec![tag];
    out.extend(n.to_be_bytes());
    out.extend(bytes);
    Ok(out)
}
fn text(tag: u8, s: &str) -> Result<Vec<u8>> {
    frame(tag, s.as_bytes())
}
fn canonical_value(v: &Value) -> Result<Vec<u8>> {
    match v {
        Value::Bool(v) => text(b'b', if *v { "true" } else { "false" }),
        Value::String(s) => text(b's', s),
        Value::Int64(s) => {
            if !s.parse::<i64>().is_ok_and(|n| n.to_string() == *s) {
                return err("TI_INPUT", "noncanonical integer");
            };
            text(b'i', s)
        }
        Value::UserId(s) => {
            id_check(s)?;
            text(b'u', s)
        }
        Value::Null => frame(b'n', &[]),
        Value::Id(id) => {
            id_check(&id.id)?;
            let mut nested = text(b't', &id.entity)?;
            nested.extend(text(b'v', &id.id)?);
            frame(b'e', &nested)
        }
        Value::State(_) => err("TI_TYPE", "state cannot be fingerprint input"),
    }
}
/// Tokens are constructed from verified host context, never request-supplied scope.
pub struct FingerprintContext<'a> {
    pub scope: &'a str,
    pub incarnation: &'a str,
    pub identity_domain: &'a str,
    pub principal: &'a str,
    pub module: &'a str,
    pub version: u64,
    pub artifact: &'a str,
    pub entity: &'a str,
    pub action: &'a str,
    pub record: Option<&'a str>,
    pub revision: Option<&'a str>,
}
pub fn fingerprint(ctx: &FingerprintContext<'_>, inputs: &Map<String, Value>) -> Result<String> {
    if ctx.version == 0
        || ctx.version > 2147483647
        || ctx.scope.is_empty()
        || ctx.incarnation.is_empty()
        || ctx.principal.is_empty()
        || ctx.identity_domain.is_empty()
        || ctx.artifact.is_empty()
    {
        return err("TI_INPUT", "invalid trusted fingerprint context");
    }
    if ctx.action == "Create" {
        if ctx.record.is_some() || ctx.revision.is_some() {
            return err("TI_INPUT", "creation requires empty record/revision");
        }
    } else if ctx.record.is_none_or(str::is_empty) || ctx.revision.is_none_or(str::is_empty) {
        return err("TI_INPUT", "record and revision required");
    }
    let mut out = vec![];
    for (tag, value) in [
        (b'd', "ti.request/0.1"),
        (b'l', "ti/0.1"),
        (b'p', "approval-core/0.1"),
        (b's', ctx.scope),
        (b'g', ctx.incarnation),
        (b'a', ctx.identity_domain),
        (b'u', ctx.principal),
        (b'm', ctx.module),
        (b'v', &ctx.version.to_string()),
        (b'h', ctx.artifact),
        (b'e', ctx.entity),
        (b'c', ctx.action),
        (b'r', ctx.record.unwrap_or("")),
        (b'q', ctx.revision.unwrap_or("")),
    ] {
        out.extend(text(tag, value)?)
    }
    let mut entries = vec![];
    for (name, value) in inputs {
        let mut entry = text(b'k', name)?;
        entry.extend(canonical_value(value)?);
        entries.extend(frame(b'p', &entry)?)
    }
    out.extend(frame(b'x', &entries)?);
    Ok(format!("{:x}", Sha256::digest(out)))
}

/// Public event encoding uses the same nominal scalar tags as Action inputs.
pub fn encode_event(event: &crate::Event) -> Result<serde_json::Value> {
    let fields: Result<Map<_, _>> = event
        .fields
        .iter()
        .map(|(name, value)| Ok((name.clone(), encode_value(value)?)))
        .collect();
    Ok(serde_json::json!({"name":event.name,"fields":fields?}))
}
