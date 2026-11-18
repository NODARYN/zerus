use super::*;
use crate::{
    state::listing_fixture as producer,
    teams::ops::{
        tests::{person, TEAM},
        Operation,
    },
};
use serde_json::json;

const FIXTURE: &str = "desktop::teams::confirm::scan::tests::unknown_fixture";

#[test]
fn a_persistently_unknown_end_gets_one_card_and_one_fault_log() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            FIXTURE,
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "run in an isolated child"]
fn unknown_fixture() {
    let mut h = crate::desktop::teams::tests::bound151_fixture();
    let state = h.temp.path().to_owned();
    std::env::set_var("HGS_STATE_DIR", &state);
    let config = Config {
        home: String::new(),
        dir: state.join("config"),
        state: state.clone(),
        selves: vec![],
        peers: vec![],
        base_peers: vec![],
        peers_from_environment: false,
        machines: Default::default(),
        tab: false,
        tab_colors: String::new(),
    };
    let (binding, name) = super::super::native::launch_binding(&config, TEAM, "m1").unwrap();
    let mut record = producer::launched_record(
        &binding.agent,
        &name,
        &binding.launch,
        &binding.session,
        &binding.run,
        &binding.folder,
        &binding.account,
    );
    record["worktree_owner"] = json!({"id":binding.nonce});
    record["pid"] = json!(i32::MAX);
    record["process_start"] = json!("1");
    producer::save(&mut record).unwrap();
    assert_eq!(
        crate::state::dispatch_test_archive_catalog_worker().unwrap(),
        0
    );
    crate::desktop::teams::child::FIXTURE_RUN.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(|_, args, _| {
            assert_eq!(args[2], "inspect");
            crate::desktop::teams::child::Ended::Exited {
                code: 1,
                output: Vec::new(),
            }
        }));
    });
    let machine = binding.machine;
    let stop = Arc::new(AtomicBool::new(false));
    let mut cards = actor::Runtime::open(&state, &machine).unwrap();
    let mut cursor = Scan::default();
    let mut logs = Vec::new();
    let mut pass = |store: &mut Store, cards: &mut actor::Runtime, cursor: &mut Scan, now| {
        step_at(
            &config,
            ("epoch", &machine),
            store,
            &stop,
            (cursor, cards),
            || now,
            |error| {
                if error.contains(&name) {
                    logs.push(error.to_owned());
                }
            },
        )
        .unwrap();
    };
    let existing = |store: &Store| {
        store
            .cards(TEAM, 200, None)
            .unwrap()
            .rows
            .into_iter()
            .filter(|card| card.kind == "team_actor_blocked" && card.text.contains(&name))
            .collect::<Vec<_>>()
    };
    let start = 1_000_000;
    for millis in [0, 4_500, 9_000, 179_999] {
        pass(&mut h.store, &mut cards, &mut cursor, start + millis);
        assert!(existing(&h.store).is_empty());
    }
    // The original read and its age survive recreation of the service adapter.
    cards = actor::Runtime::open(&state, &machine).unwrap();
    cursor = Scan::default();
    pass(&mut h.store, &mut cards, &mut cursor, start + 180_000);
    pass(&mut h.store, &mut cards, &mut cursor, start + 184_500);
    let first = existing(&h.store);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].state, "open");
    assert!(first[0]
        .text
        .contains("ended is unknown (No exact unique exit archive)"));
    assert_eq!(first[0].decisions, ["retry", "acknowledged"]);
    assert!(!state.join("archive-catalog-v2/teams-attempt.json").exists());
    assert!(
        !state.join("approval-audit").exists(),
        "unknown did not reserve an original"
    );
    let mut unrelated = producer::launched_record(
        "codex",
        "codex/example/unrelated",
        &uuid::Uuid::new_v4().to_string(),
        &uuid::Uuid::new_v4().to_string(),
        "different-run",
        "/fixture/worktree",
        "test-account",
    );
    producer::confirmed(&mut unrelated, "different-conversation");
    producer::save(&mut unrelated).unwrap();
    producer::archive_with_catalog_gap(&unrelated).unwrap();
    pass(&mut h.store, &mut cards, &mut cursor, start + 189_000);
    pass(&mut h.store, &mut cards, &mut cursor, start + 193_500);
    assert_eq!(existing(&h.store)[0].seq, first[0].seq);
    pass(&mut h.store, &mut cards, &mut cursor, start + 369_000);
    let replaced = existing(&h.store);
    assert!(replaced
        .iter()
        .any(|card| card.seq == first[0].seq && card.state == "closed"));
    assert_eq!(
        replaced.iter().filter(|card| card.state == "open").count(),
        1
    );

    h.run(
        &person(),
        Operation::StopMember {
            member: "m1".into(),
        },
    )
    .unwrap();
    // The bounded roster cursor can spend a pass reaching the end of the
    // team page before it starts a fresh sweep.
    pass(&mut h.store, &mut cards, &mut cursor, start + 373_500);
    pass(&mut h.store, &mut cards, &mut cursor, start + 378_000);
    assert!(existing(&h.store).iter().all(|card| card.state == "closed"));
    assert_eq!(logs.len(), 2, "{logs:?}");
    assert!(logs[0].contains("No exact unique exit archive"));
    assert!(logs[1].contains("Archive catalog coverage is incomplete"));
}
