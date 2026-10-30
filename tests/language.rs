use std::collections::{BTreeMap as Map, BTreeSet};
use ti_lang::{compile, Actor, Value};
const APPROVAL: &str = include_str!("../examples/approval.ti");
fn actor(id: &str, role: &str) -> Actor {
    Actor {
        id: id.into(),
        roles: BTreeSet::from([role.into()]),
    }
}
fn inputs(days: &str) -> Map<String, Value> {
    Map::from([
        ("reason".into(), Value::String("  Family leave  ".into())),
        ("days".into(), Value::Int64(days.into())),
    ])
}
#[test]
fn complete_approval_and_denial() {
    let m = compile(APPROVAL).unwrap();
    let applicant = actor("alice", "leave.applicant");
    let reviewer = actor("bob", "leave.reviewer");
    let c = m
        .evaluate(
            "LeaveRequest",
            "Create",
            &applicant,
            &inputs("3"),
            None,
            "request-1",
        )
        .unwrap();
    assert_eq!(c.record.state, "Draft");
    assert_eq!(
        c.record.fields["reason"],
        Value::String("Family leave".into())
    );
    assert_eq!(c.record.fields["rejectionReason"], Value::Null);
    let p = m
        .evaluate(
            "LeaveRequest",
            "Submit",
            &applicant,
            &Map::new(),
            Some(&c.record),
            "",
        )
        .unwrap();
    assert_eq!(p.record.state, "Pending");
    assert_eq!(
        m.evaluate(
            "LeaveRequest",
            "Approve",
            &actor("alice", "leave.reviewer"),
            &Map::new(),
            Some(&p.record),
            ""
        )
        .unwrap_err()
        .code,
        "TI_ACCESS_DENIED"
    );
    let done = m
        .evaluate(
            "LeaveRequest",
            "Approve",
            &reviewer,
            &Map::new(),
            Some(&p.record),
            "",
        )
        .unwrap();
    assert_eq!(done.record.state, "Approved");
    assert_eq!(done.events.len(), 1);
    assert_eq!(done.events[0].fields["days"], Value::Int64("3".into()));
    assert_eq!(p.record.state, "Pending");
    assert_eq!(
        m.evaluate(
            "LeaveRequest",
            "Approve",
            &reviewer,
            &Map::new(),
            Some(&done.record),
            ""
        )
        .unwrap_err()
        .code,
        "TI_STATE"
    );
}
#[test]
fn invalid_input_invariant_and_requirement() {
    let m = compile(APPROVAL).unwrap();
    let a = actor("a", "leave.applicant");
    assert_eq!(
        m.evaluate("LeaveRequest", "Create", &a, &inputs("0"), None, "1")
            .unwrap_err()
            .code,
        "TI_INVARIANT"
    );
    assert_eq!(
        m.evaluate("LeaveRequest", "Create", &a, &inputs("01"), None, "1")
            .unwrap_err()
            .code,
        "TI_INPUT"
    );
    let c = m
        .evaluate("LeaveRequest", "Create", &a, &inputs("1"), None, "1")
        .unwrap();
    let p = m
        .evaluate(
            "LeaveRequest",
            "Submit",
            &a,
            &Map::new(),
            Some(&c.record),
            "",
        )
        .unwrap();
    assert_eq!(
        m.evaluate(
            "LeaveRequest",
            "Reject",
            &actor("b", "leave.reviewer"),
            &Map::from([("reason".into(), Value::String(" \t".into()))]),
            Some(&p.record),
            ""
        )
        .unwrap_err()
        .code,
        "TI_REQUIRE"
    );
}
#[test]
fn simultaneous_assignments_and_candidate_event() {
    let s="module swap version 1; entity Pair { field a: Int64; field b: Int64; state status { Ready initial; } } create Pair() { allow true; set a = 1; set b = 2; } action Swap on Pair { from Ready to Ready; allow true; set a = self.b; set b = self.a; emit Changed { a = self.a; } } event Changed { field a: Int64; }";
    let m = compile(s).unwrap();
    let a = actor("a", "x");
    let old = m
        .evaluate("Pair", "Create", &a, &Map::new(), None, "p")
        .unwrap();
    let new = m
        .evaluate("Pair", "Swap", &a, &Map::new(), Some(&old.record), "")
        .unwrap();
    assert_eq!(new.record.fields["a"], Value::Int64("2".into()));
    assert_eq!(new.record.fields["b"], Value::Int64("1".into()));
    assert_eq!(new.events[0].fields["a"], Value::Int64("2".into()));
}
#[test]
fn rejects_static_errors() {
    for bad in [
        APPROVAL.replace("allow Reviewer(self);", ""),
        APPROVAL.replace(
            "set days = input.days;",
            "set days = input.days; set days = 2;",
        ),
        APPROVAL.replace("from Pending to Approved;", "from Approved to Pending;"),
        APPROVAL.replace("days > 0", "actor.hasRole(\"x\")"),
        APPROVAL.replace("days = self.days;", "days = nonempty(self.reason);"),
        APPROVAL.replace("field days: Int64;", "field days: Int64??;"),
        APPROVAL.replace("days > 0", "days > 0 > 1"),
        APPROVAL.replace("days > 0", "Draft == Pending"),
        APPROVAL.replace("actor.id == request.applicant;", "Applicant(request);"),
    ] {
        assert!(compile(&bad).is_err(), "accepted invalid program: {bad}");
    }
}
#[test]
fn rejects_mutual_policy_cycle() {
    let s = APPROVAL
        .replace("actor.id == request.applicant;", "Reviewer(request);")
        .replace(
            "actor.hasRole(\"leave.reviewer\") and actor.id != request.applicant;",
            "Applicant(request);",
        );
    assert_eq!(compile(&s).unwrap_err().code, "TI_EFFECT");
}
#[test]
fn lexical_limits_and_unicode() {
    assert!(compile(&format!("\u{feff}{APPROVAL}")).is_err());
    assert!(compile(&APPROVAL.replace("version 1;", "version 01;")).is_err());
    assert!(compile(&APPROVAL.replace("days > 0", "days > 9223372036854775808")).is_err());
    assert!(compile(&APPROVAL.replace(
        "days > 0",
        &format!("{}true{}", "(".repeat(100), ")".repeat(100))
    ))
    .is_err());
    assert!(compile(&APPROVAL.replace("days > 0", "length(\"😀\") == 1")).is_ok());
}
#[test]
fn bounded_policy_graph_does_not_expand_exponentially() {
    let mut s="module test version 1; entity Item { state status { Ready initial; } } create Item() { allow true; } policy P0(x: Item) = true;".to_string();
    for i in 1..80 {
        s += &format!("policy P{i}(x: Item) = P{}(x) and P{}(x);", i - 1, i - 1)
    }
    s += "action Test on Item { from Ready to Ready; allow P79(self); }";
    let m = compile(&s).unwrap();
    let a = actor("a", "x");
    let c = m
        .evaluate("Item", "Create", &a, &Map::new(), None, "1")
        .unwrap();
    assert_eq!(
        m.evaluate("Item", "Test", &a, &Map::new(), Some(&c.record), "")
            .unwrap_err()
            .code,
        "TI_LIMIT"
    );
}

#[test]
fn remaining_approval_branches_and_old_record_validation() {
    let m = compile(APPROVAL).unwrap();
    let a = actor("alice", "leave.applicant");
    let c = m
        .evaluate("LeaveRequest", "Create", &a, &inputs("3"), None, "r")
        .unwrap();
    let edited = m
        .evaluate(
            "LeaveRequest",
            "EditDraft",
            &a,
            &inputs("5"),
            Some(&c.record),
            "",
        )
        .unwrap();
    assert_eq!(edited.record.fields["days"], Value::Int64("5".into()));
    let p = m
        .evaluate(
            "LeaveRequest",
            "Submit",
            &a,
            &Map::new(),
            Some(&edited.record),
            "",
        )
        .unwrap();
    let rejected = m
        .evaluate(
            "LeaveRequest",
            "Reject",
            &actor("bob", "leave.reviewer"),
            &Map::from([("reason".into(), Value::String("  Not available  ".into()))]),
            Some(&p.record),
            "",
        )
        .unwrap();
    assert_eq!(rejected.record.state, "Rejected");
    assert_eq!(
        rejected.record.fields["rejectionReason"],
        Value::String("Not available".into())
    );
    let withdrawn = m
        .evaluate(
            "LeaveRequest",
            "Withdraw",
            &a,
            &Map::new(),
            Some(&p.record),
            "",
        )
        .unwrap();
    assert_eq!(withdrawn.record.state, "Withdrawn");
    let mut invalid = c.record.clone();
    invalid
        .fields
        .insert("days".into(), Value::Int64("0".into()));
    assert_eq!(
        m.evaluate(
            "LeaveRequest",
            "Submit",
            &a,
            &Map::new(),
            Some(&invalid),
            ""
        )
        .unwrap_err()
        .code,
        "TI_RECORD_INVALID"
    );
}
#[test]
fn flat_expression_chains_are_bounded() {
    let source = APPROVAL.replace(
        "days > 0",
        &std::iter::repeat_n("true", 10000)
            .collect::<Vec<_>>()
            .join(" and "),
    );
    assert_eq!(compile(&source).unwrap_err().code, "TI_LIMIT");
}
#[test]
fn cli_compiles_and_evaluates_checked_model() {
    let check = std::process::Command::new(env!("CARGO_BIN_EXE_ti"))
        .args(["check", "examples/approval.ti"])
        .output()
        .unwrap();
    assert!(check.status.success());
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ti"))
        .args([
            "eval",
            "examples/approval.ti",
            "examples/create-request.json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["record"]["state"], "Draft");
}
