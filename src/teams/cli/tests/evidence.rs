//! zerus-5gcbh: the evidence beside a task's decision. `evidence --team ID
//! --node N` reads the task's latest report and each of its stages with the
//! stage's latest verdict (who gave it, pass or return, its findings), from
//! the rows the report and verdict writers stored; the person and the lead
//! read it, a member does not.
use super::harness::*;
use super::verdict::{configured_stage, REVIEWER_SESSION};
use crate::teams::cli::actor::Caller;
use serde_json::{json, Value};

fn lead() -> Caller {
    Caller::Session(key(LEAD_SESSION))
}
fn reviewer() -> Caller {
    Caller::Session(key(REVIEWER_SESSION))
}
/// The reviewer's verdict through the product writer (`hgs team verdict`).
fn verdict(h: &Harness, node: u32, pass: bool, text: &str, id: u32) {
    let input = json!({"request_id":uid(id),"team":TEAM,"expected_revision":h.revision(),"node":node,"generation":1,"pass":pass,"text":text});
    let (reply, exit) = h.cli(&reviewer(), "verdict", &input.to_string());
    assert_eq!(exit, 0, "{reply}");
}
fn evidence(h: &Harness, caller: &Caller, node: u32) -> (Value, i32) {
    h.cli(caller, &format!("evidence --team {TEAM} --node {node}"), "")
}

/// The reply the desktop reads once the review passed and the task waits for
/// its decision; also pinned as `cli-evidence.json`.
pub(super) fn passed_review_reply() -> Value {
    let (h, stage) = configured_stage(
        zerus_team_plan::catalog::StageKind::Review,
        3,
        Some("example-model"),
    );
    verdict(
        &h,
        stage,
        true,
        "Reviewed every acceptance criterion. One nit: rename the helper.",
        0x8100,
    );
    let (reply, exit) = evidence(&h, &lead(), 1);
    assert_eq!(exit, 0, "{reply}");
    reply
}

#[test]
fn evidence_names_the_report_and_the_reviewers_verdict_beside_the_decision() {
    let (h, stage) = configured_stage(
        zerus_team_plan::catalog::StageKind::Review,
        3,
        Some("example-model"),
    );
    verdict(
        &h,
        stage,
        true,
        "Reviewed every acceptance criterion. One nit: rename the helper.",
        0x8100,
    );
    for caller in [lead(), Caller::Person] {
        let (reply, exit) = evidence(&h, &caller, 1);
        assert_eq!(exit, 0, "{reply}");
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["team"], TEAM);
        assert_eq!(reply["node"], 1);
        assert_eq!(reply["revision"], h.revision());
        let report = &reply["report"];
        assert_eq!(report["generation"], 1);
        assert_eq!(report["member"], "m1");
        assert_eq!(report["commit"], COMMIT);
        assert_eq!(
            report["text"],
            "Trimmed the label and added a case for an empty one."
        );
        assert_eq!(report["notes"], json!([]));
        assert_eq!(reply["more_stages"], false);
        let stages = reply["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 1, "{reply}");
        assert_eq!(stages[0]["node"], stage);
        assert_eq!(stages[0]["kind"], "review");
        assert_eq!(stages[0]["round"], 1);
        // The verdict is recorded; the stage node keeps its stored state until
        // the service observes it, and the evidence never invents another.
        assert_eq!(stages[0]["state"], "in_progress");
        assert_eq!(
            stages[0]["verdict"],
            json!({"generation":1,"member":"m3","pass":true,"text":"Reviewed every acceptance criterion. One nit: rename the helper."})
        );
    }
}

#[test]
fn a_returned_review_keeps_its_findings_and_who_gave_them() {
    let (h, stage) = configured_stage(
        zerus_team_plan::catalog::StageKind::Review,
        3,
        Some("example-model"),
    );
    verdict(
        &h,
        stage,
        false,
        "The empty label still overflows; add a test.",
        0x8101,
    );
    let (reply, exit) = evidence(&h, &Caller::Person, 1);
    assert_eq!(exit, 0, "{reply}");
    let verdict = &reply["stages"][0]["verdict"];
    assert_eq!(verdict["member"], "m3");
    assert_eq!(verdict["pass"], false);
    assert_eq!(
        verdict["text"],
        "The empty label still overflows; add a test."
    );
}

#[test]
fn a_task_without_report_or_stages_has_no_invented_evidence() {
    let h = seeded();
    let node = h.add(7);
    let (reply, exit) = evidence(&h, &Caller::Person, node);
    assert_eq!(exit, 0, "{reply}");
    assert_eq!(reply["report"], Value::Null);
    assert_eq!(reply["stages"], json!([]));
    assert_eq!(reply["more_stages"], false);
}

#[test]
fn evidence_refuses_a_stage_an_unknown_node_a_member_and_bad_flags() {
    let (h, stage) = configured_stage(
        zerus_team_plan::catalog::StageKind::Review,
        3,
        Some("example-model"),
    );
    let (reply, exit) = evidence(&h, &Caller::Person, stage);
    assert_eq!((code(&reply), exit), ("invalid_state", 1), "{reply}");
    let (reply, exit) = evidence(&h, &Caller::Person, 999);
    assert_eq!((code(&reply), exit), ("not_found", 1), "{reply}");
    let (reply, exit) = evidence(&h, &Caller::Session(key(M1_SESSION)), 1);
    assert_eq!((code(&reply), exit), ("not_allowed", 1), "{reply}");
    for line in [
        format!("evidence --team {TEAM}"),
        format!("evidence --team {TEAM} --node x"),
        format!("evidence --team {TEAM} --node 1 --limit 5"),
    ] {
        let (reply, exit) = h.cli(&Caller::Person, &line, "");
        assert_eq!(
            (code(&reply), exit),
            ("invalid_state", 1),
            "{line}: {reply}"
        );
    }
}

#[test]
fn evidence_is_a_listed_read() {
    assert!(crate::teams::cli::USAGE.contains("evidence --team ID --node N"));
}
