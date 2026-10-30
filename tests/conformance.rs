//! Stable V/E conformance IDs. Host cases live in the native host suite.
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use ti_lang::{compile, Actor, Value};
const MODEL: &str = include_str!("../examples/approval.ti");
fn check(s: &str) {
    compile(s).unwrap_or_else(|e| panic!("{e:?} in {s}"));
}
fn reject(s: &str) {
    assert!(compile(s).is_err(), "accepted invalid source");
}
fn model(expr: &str) -> String {
    format!("module sample version 1; entity Item {{ field value: String?; state status {{ Draft initial; Done final; }} invariant test: {expr}; }} create Item() {{ allow true; }} action Finish on Item {{ from Draft to Done; allow true; }}")
}
fn create(s: &str) -> ti_lang::Result<ti_lang::Candidate> {
    compile(s)?.evaluate(
        "Item",
        "Create",
        &Actor {
            id: "a".into(),
            roles: Set::new(),
        },
        &Map::new(),
        None,
        "1",
    )
}
#[test]
fn tc_001() {
    check(MODEL)
}
#[test]
fn tc_002() {
    assert_eq!(
        ti_lang::compile_bytes(&[0xff]).unwrap_err().code,
        "TI_SYNTAX"
    );
    reject(&format!("\u{feff}{MODEL}"));
    reject(&MODEL.replace('\n', "\r"));
}
#[test]
fn tc_003() {
    reject(&MODEL.replace("module", "Module"));
    reject(&MODEL.replace("LeaveRequest", &"a".repeat(64)));
    reject(&format!("{MODEL} ;"));
}
#[test]
fn tc_004() {
    check(&model(r#"length("\uD83D\uDE00") == 1"#));
    reject(&model(r#"length("\uD800") == 1"#));
    reject(&model("length(\"a\nb\") == 3"));
    assert_eq!(
        create(&model("\"é\" == \"é\"")).unwrap_err().code,
        "TI_INVARIANT"
    );
}
#[test]
fn tc_005() {
    for n in ["-9223372036854775808", "9223372036854775807", "0"] {
        check(&model(&format!("{n} == {n}")));
    }
    for n in [
        "9223372036854775808",
        "-9223372036854775809",
        "01",
        "-0",
        "+1",
    ] {
        reject(&model(&format!("{n} == 0")));
    }
}
#[test]
fn tc_006() {
    reject(&MODEL.replace(
        "field reason: String;",
        "field reason: String; field reason: String;",
    ));
    reject(&MODEL.replace("create LeaveRequest", "create Missing"));
    reject(&format!("{MODEL} event LeaveRequestId {{}}"));
}
#[test]
fn tc_007() {
    reject(&MODEL.replace("set applicant = actor.id;", "set applicant = \"a\";"));
    reject(&MODEL.replace("field days: Int64;", "field days: LeaveRequestId;"));
}
#[test]
fn tc_008() {
    let base = model("value == null");
    assert!(create(&base).is_ok());
    let nullable = base
        .replace("allow true; }", "allow true; set value = \"text\"; }")
        .replace("value == null", "nonempty(value)");
    assert!(create(&nullable).is_ok());
    reject(&base.replace("String?", "String??"));
    reject(&MODEL.replace("set reason = trim(input.reason);", "set reason = null;"));
}
#[test]
fn tc_009() {
    for x in ["value == \"x\"", "value == value", "value", "\"a\" < \"b\""] {
        reject(&model(x));
    }
}
#[test]
fn tc_010() {
    for x in [
        "length(\"😀\") == 1",
        "trim(\" \\tA\\r\\n\") == \"A\"",
        "trim(\" \") == \" \"",
        "not nonempty(null)",
    ] {
        assert!(create(&model(x)).is_ok(), "{x}");
    }
}
#[test]
fn tc_011() {
    reject(&MODEL.replace(
        "actor.hasRole(\"leave.applicant\")",
        "actor.hasRole(input.reason)",
    ));
    reject(&MODEL.replace("days > 0", "actor.hasRole(\"x\")"));
    reject(&MODEL.replace("set days = input.days;", "set days = self.days;"));
    reject(&MODEL.replace("days = self.days;", "days = actor.id;"));
}
#[test]
fn tc_012() {
    assert!(create(&model("true or false and false")).is_ok());
    assert!(create(&model("not (1 == 2)")).is_ok());
    reject(&model("1 < 2 < 3"));
    reject(&model("true or unknown"));
}
#[test]
fn tc_013() {
    reject(&format!(
        "{MODEL} policy Cycle(x: LeaveRequest) = Cycle(x);"
    ));
    reject(&MODEL.replace("request.applicant", "request.applicant.name"));
}
#[test]
fn tc_014() {
    reject(&MODEL.replace("Pending;", "Pending initial;"));
    reject(&MODEL.replace("from Pending to Approved;", "from Approved to Pending;"));
    reject(&MODEL.replace("state status", "field status"));
}
#[test]
fn tc_015() {
    let s = model("true") + " action Other on Item { from Draft to Draft; allow true; }";
    let m = compile(&s).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "1")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Other", &a, &Map::new(), Some(&old.record), "")
            .unwrap()
            .record
            .state,
        "Draft"
    );
    assert_eq!(
        m.evaluate("Item", "Finish", &a, &Map::new(), Some(&old.record), "")
            .unwrap()
            .record
            .state,
        "Done"
    );
}
#[test]
fn tc_016() {
    assert_eq!(
        create(&model("true")).unwrap().record.fields["value"],
        Value::Null
    );
    reject(&MODEL.replace("set days = input.days;", ""));
}
#[test]
fn tc_017() {
    for s in [
        MODEL.replace("allow Reviewer(self);", ""),
        MODEL.replace(
            "set days = input.days;",
            "set days = input.days; set days = 2;",
        ),
        MODEL.replace("set days = input.days;", "set status = Draft;"),
        MODEL.replace("set days = input.days;", "set id = \"1\";"),
        MODEL.replace(
            "require nonempty(input.reason) else \"rejection_reason_required\";",
            "require true else \"same\"; require false else \"same\";",
        ),
    ] {
        reject(&s);
    }
}
#[test]
fn tc_018() {
    let s = model("true").replace("from Draft to Done;", "from Draft to Draft;");
    let m = compile(&s).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "1")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Finish", &a, &Map::new(), Some(&old.record), "")
            .unwrap()
            .record
            .state,
        "Draft"
    );
}
#[test]
fn tc_019() {
    assert_eq!(create(&model("false")).unwrap_err().code, "TI_INVARIANT");
    let s = model("true").replace(
        "from Draft to Done; allow true;",
        "from Draft to Done; allow true; require false else \"failed\";",
    );
    let m = compile(&s).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "1")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Finish", &a, &Map::new(), Some(&old.record), "")
            .unwrap_err()
            .code,
        "TI_REQUIRE"
    );
}
#[test]
fn tc_020() {
    for s in [
        MODEL.replace("days = self.days;", ""),
        MODEL.replace("days = self.days;", "days = self.days; days = self.days;"),
        MODEL.replace("days = self.days;", "unknown = self.days;"),
    ] {
        reject(&s);
    }
}
#[test]
fn tc_021() {
    let mut s = model("true");
    s += " policy P0(x: Item) = true;";
    for i in 1..20 {
        s += &format!("policy P{i}(x: Item) = P{}(x) and P{}(x);", i - 1, i - 1)
    }
    s = s.replace(
        "from Draft to Done; allow true;",
        "from Draft to Done; allow P19(self);",
    );
    let m = compile(&s).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let c = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "1")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Finish", &a, &Map::new(), Some(&c.record), "")
            .unwrap_err()
            .code,
        "TI_LIMIT"
    );
}
#[test]
fn tc_039_validator_evaluator() {
    check(&model(&format!(
        "length(\"{}\") == 16384",
        "a".repeat(16384)
    )));
    reject(&model(&format!(
        "length(\"{}\") == 16385",
        "a".repeat(16385)
    )));
    reject(&" ".repeat(262145));
}
#[test]
fn tc_040() {
    for s in ["import sql;", "loop {}", "func test() {}"] {
        reject(&format!("{MODEL} {s}"));
    }
    reject(&MODEL.replace("field days: Int64;", "field days: Decimal;"));
    reject(&model("http.get(\"x\")"));
}
#[test]
fn tc_041() {
    use ti_lang::wire::decode_inputs;
    for b in [
        r#"{"x":{"type":"Int64","value":9007199254740993}}"#,
        r#"{"x":{"type":"Int64","value":"1"},"x":{"type":"Int64","value":"2"}}"#,
        r#"{"x":{"type":"Int64","type":"String","value":"1"}}"#,
        r#"{"x":{"type":"null","value":null}}"#,
        r#"{"x":{"type":"native-record","value":"user:a"}}"#,
    ] {
        assert!(decode_inputs(b.as_bytes()).is_err());
    }
    let n = decode_inputs(br#"{"x":{"type":"LeaveRequestId","value":"1"}}"#).unwrap();
    assert!(matches!(&n["x"],Value::Id(v)if v.entity=="LeaveRequest"));
}
#[test]
fn tc_042() {
    use ti_lang::wire::*;
    assert_eq!(frame(b'i', b"3").unwrap(), [0x69, 0, 0, 0, 1, 0x33]);
    let ctx = FingerprintContext {
        scope: "scope-v1",
        incarnation: "1",
        identity_domain: "app",
        principal: "a",
        module: "leave",
        version: 1,
        artifact: "frozen",
        entity: "LeaveRequest",
        action: "Create",
        record: None,
        revision: None,
    };
    let a = decode_inputs(
        br#"{"x":{"type":"String","value":"a:b"},"y":{"type":"String","value":"c"}}"#,
    )
    .unwrap();
    let b = decode_inputs(
        br#"{"y":{"type":"String","value":"c"},"x":{"type":"String","value":"a:b"}}"#,
    )
    .unwrap();
    let c = decode_inputs(
        br#"{"x":{"type":"String","value":"a"},"y":{"type":"String","value":"b:c"}}"#,
    )
    .unwrap();
    assert_eq!(
        fingerprint(&ctx, &a).unwrap(),
        fingerprint(&ctx, &b).unwrap()
    );
    assert_ne!(
        fingerprint(&ctx, &a).unwrap(),
        fingerprint(&ctx, &c).unwrap()
    );
}

#[test]
fn tc_036_validator_artifact() {
    use ti_lang::artifact::*;
    let first = Artifact::freeze(MODEL, Map::from([("schema".into(), "1".into())])).unwrap();
    let same = Artifact::freeze(MODEL, Map::from([("schema".into(), "1".into())])).unwrap();
    assert_eq!(first.identity(), same.identity());
    let changed = Artifact::freeze(
        &format!("{MODEL}\n// changed"),
        Map::from([("schema".into(), "1".into())]),
    )
    .unwrap();
    assert_ne!(first.identity(), changed.identity());
    let mut registry = Registry::default();
    registry.publish(first).unwrap();
    registry.publish(same).unwrap();
    assert!(registry.publish(changed).is_err());
}
#[test]
fn tc_039_static_boundaries() {
    let base="module limits version 1; entity Item { state status { Draft initial; } } create Item() { allow true; }";
    // Exactly 256 declarations (entity + creation + 254 empty events).
    for count in [254, 255] {
        let mut s = base.to_owned();
        for i in 0..count {
            s += &format!(" event E{i} {{ }}");
        }
        if count == 254 {
            check(&s)
        } else {
            reject(&s)
        }
    }
    for count in [64, 65] {
        let fields = (0..count)
            .map(|i| format!("field f{i}: String?;"))
            .collect::<String>();
        let states = (1..count).map(|i| format!(" S{i};")).collect::<String>();
        let invariants = (0..count)
            .map(|i| format!(" invariant I{i}: true;"))
            .collect::<String>();
        let inputs = (0..count)
            .map(|i| format!("p{i}: String?"))
            .collect::<Vec<_>>()
            .join(",");
        let requires = (0..count)
            .map(|i| format!(" require true else \"r{i}\";"))
            .collect::<String>();
        for source in [
            base.replace("state status", &format!("{fields} state status")),
            base.replace("Draft initial;", &format!("Draft initial;{states}")),
            base.replace("} } create", &format!("}} {invariants} }} create")),
            base.replace("create Item()", &format!("create Item({inputs})")),
            base.replace("allow true;", &format!("allow true;{requires}")),
            format!("{base} event E {{ {fields} }}"),
        ] {
            if count == 64 {
                check(&source)
            } else {
                reject(&source)
            }
        }
    }
    for count in [16, 17] {
        let emits = " emit E {}".repeat(count);
        let source = format!(
            "{base} event E {{}} action Run on Item {{from Draft to Draft; allow true; {emits}}}"
        );
        if count == 16 {
            check(&source)
        } else {
            reject(&source)
        }
    }
    check(&base.replace("limits", &"a".repeat(63)));
    reject(&base.replace("limits", &"a".repeat(64)));
    check(&format!("{base}{}", " ".repeat(262144 - base.len())));
    reject(&format!("{base}{}", " ".repeat(262145 - base.len())));
}
#[test]
fn tc_013_mutual_recursion() {
    reject(&format!(
        "{MODEL} policy A(x: LeaveRequest) = B(x); policy B(x: LeaveRequest) = A(x);"
    ));
}
#[test]
fn tc_018_swap() {
    let source="module swap version 1; entity Item {field a: String; field b: String; state status {Draft initial;}} create Item(){allow true; set a=\"a\"; set b=\"b\";} action Swap on Item {from Draft to Draft;allow true;set a=self.b;set b=self.a;}";
    let m = compile(source).unwrap();
    let actor = Actor {
        id: "actor".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &actor, &Map::new(), None, "one")
        .unwrap();
    let new = m
        .evaluate("Item", "Swap", &actor, &Map::new(), Some(&old.record), "")
        .unwrap();
    assert_eq!(new.record.fields["a"], Value::String("b".into()));
    assert_eq!(new.record.fields["b"], Value::String("a".into()));
}
#[test]
fn tc_019_old_and_candidate_invariants() {
    let s="module inv version 1; entity Item{field n:Int64; state status{Draft initial;}invariant positive:n>0;} create Item(){allow true;set n=1;} action Bad on Item{from Draft to Draft;allow true;set n=0;}";
    let m = compile(s).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "one")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Bad", &a, &Map::new(), Some(&old.record), "")
            .unwrap_err()
            .code,
        "TI_INVARIANT"
    );
    let mut invalid = old.record;
    invalid.fields.insert("n".into(), Value::Int64("0".into()));
    assert_eq!(
        m.evaluate("Item", "Bad", &a, &Map::new(), Some(&invalid), "")
            .unwrap_err()
            .code,
        "TI_RECORD_INVALID"
    );
}
#[test]
fn tc_020_candidate_event_value() {
    let source="module events version 1; entity Item{field n:Int64; state status{Draft initial;}} create Item(){allow true;set n=1;} action Change on Item{from Draft to Draft;allow true;set n=2;emit Changed{n=self.n;}} event Changed{field n:Int64;}";
    let m = compile(source).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "one")
        .unwrap();
    let new = m
        .evaluate("Item", "Change", &a, &Map::new(), Some(&old.record), "")
        .unwrap();
    assert_eq!(new.events[0].fields["n"], Value::Int64("2".into()));
}
#[test]
fn tc_039_runtime_boundaries() {
    let s = model("true");
    let m = compile(&s).unwrap();
    for size in [256, 257] {
        let a = Actor {
            id: "a".repeat(size),
            roles: Set::new(),
        };
        assert_eq!(
            m.evaluate("Item", "Create", &a, &Map::new(), None, "one")
                .is_ok(),
            size == 256
        );
    }
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    for size in [256, 257] {
        assert_eq!(
            m.evaluate("Item", "Create", &a, &Map::new(), None, &"x".repeat(size))
                .is_ok(),
            size == 256
        );
    }
}

#[test]
fn tc_039_expression_depth() {
    check(&model(&format!("{}true", "not ".repeat(63))));
    reject(&model(&format!("{}true", "not ".repeat(64))));
}
#[test]
fn tc_021_exact_shared_budget() {
    fn policy(cost: usize, out: &mut String) -> String {
        let name = format!("P{cost}");
        let expr = if cost <= 4 {
            format!(
                "{}{}",
                "not ".repeat(cost - 1),
                if cost % 2 == 1 { "true" } else { "false" }
            )
        } else {
            let child = policy((cost - 3) / 2, out);
            let equality = format!(
                "{child}(x) {} {child}(x)",
                if cost % 2 == 1 { "==" } else { "!=" }
            );
            if cost % 2 == 1 {
                equality
            } else {
                format!("not ({equality})")
            }
        };
        *out += &format!(" policy {name}(x:Item)={expr};");
        name
    }
    for cost in [99997, 99998] {
        let mut policies = String::new();
        let name = policy(cost, &mut policies);
        let source = format!(
            "{} {policies}",
            model("true").replace(
                "from Draft to Done; allow true;",
                &format!("from Draft to Done; allow {name}(self);")
            )
        );
        let m = compile(&source).unwrap();
        let a = Actor {
            id: "a".into(),
            roles: Set::new(),
        };
        let old = m
            .evaluate("Item", "Create", &a, &Map::new(), None, "one")
            .unwrap();
        let result = m.evaluate("Item", "Finish", &a, &Map::new(), Some(&old.record), "");
        if cost == 99997 {
            assert!(result.is_ok())
        } else {
            assert_eq!(result.unwrap_err().code, "TI_LIMIT")
        }
    }
}
#[test]
fn tc_039_deep_acyclic_policy() {
    let mut source = model("true");
    source += " policy P0(x:Item)=true;";
    for i in 1..250 {
        source += &format!(" policy P{i}(x:Item)={}P{}(x);", "not ".repeat(20), i - 1);
    }
    source = source.replace(
        "from Draft to Done; allow true;",
        "from Draft to Done; allow P249(self);",
    );
    let m = compile(&source).unwrap();
    let a = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let old = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "one")
        .unwrap();
    assert!(m
        .evaluate("Item", "Finish", &a, &Map::new(), Some(&old.record), "")
        .is_ok());
}
#[test]
fn tc_039_encoded_payload_limits() {
    let actor = Actor {
        id: "a".into(),
        roles: Set::new(),
    };
    let inputs = (0..8)
        .map(|i| format!("p{i}:String"))
        .collect::<Vec<_>>()
        .join(",");
    let source=format!("module bytes version 1; entity Item{{state status{{Draft initial;}}}} create Item({inputs}){{allow true;}}");
    let m = compile(&source).unwrap();
    let mut values: Map<_, _> = (0..8)
        .map(|i| (format!("p{i}"), Value::String("a".repeat(16384))))
        .collect();
    let encoded: Map<_, _> = values
        .iter()
        .map(|(k, v)| (k, ti_lang::wire::encode_value(v).unwrap()))
        .collect();
    let overhead = serde_json::to_vec(&encoded).unwrap().len() - 131072;
    values.insert("p0".into(), Value::String("a".repeat(16384 - overhead)));
    assert!(m
        .evaluate("Item", "Create", &actor, &values, None, "one")
        .is_ok());
    values.insert("p0".into(), Value::String("a".repeat(16385 - overhead)));
    assert_eq!(
        m.evaluate("Item", "Create", &actor, &values, None, "one")
            .unwrap_err()
            .code,
        "TI_LIMIT"
    );
    let mut event = ti_lang::Event {
        name: "E".into(),
        fields: (0..8)
            .map(|i| (format!("f{i}"), Value::String("a".repeat(16384))))
            .collect(),
    };
    let overhead = serde_json::to_vec(&vec![ti_lang::wire::encode_event(&event).unwrap()])
        .unwrap()
        .len()
        - 131072;
    for extra in [0, 1] {
        event.fields.insert(
            "f0".into(),
            Value::String("a".repeat(16384 - overhead + extra)),
        );
        let fields = (0..8)
            .map(|i| format!("field f{i}:String;"))
            .collect::<String>();
        let emits = event
            .fields
            .iter()
            .map(|(k, v)| {
                let Value::String(s) = v else { panic!() };
                format!("{k}=\"{s}\";")
            })
            .collect::<String>();
        let source=format!("module payload version 1; entity Item{{state status{{Draft initial;}}}} create Item(){{allow true;}} event E{{{fields}}} action Send on Item{{from Draft to Draft;allow true;emit E{{{emits}}}}}");
        let m = compile(&source).unwrap();
        let old = m
            .evaluate("Item", "Create", &actor, &Map::new(), None, "one")
            .unwrap();
        let result = m.evaluate("Item", "Send", &actor, &Map::new(), Some(&old.record), "");
        if extra == 0 {
            assert!(result.is_ok())
        } else {
            assert_eq!(result.unwrap_err().code, "TI_LIMIT")
        }
    }
}
