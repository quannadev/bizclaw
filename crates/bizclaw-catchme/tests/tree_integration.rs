//! Integration tests for the CatchMe Tree Organizer.
//!
//! Tests the full pipeline: events → clustering → tree + persistence.

use bizclaw_catchme::event::{CatchMeEvent, EventType};
use bizclaw_catchme::store::CatchMeStore;
use bizclaw_catchme::tree::{OrganizerConfig, TreeLevel, TreeOrganizer};
use chrono::{Duration, Utc};

fn make_event(source: &str, event_type: EventType, offset_secs: i64) -> CatchMeEvent {
    CatchMeEvent {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now() + Duration::seconds(offset_secs),
        source: source.into(),
        event_type,
    }
}

/// Full pipeline: events → organize → persist → query back.
#[test]
fn test_full_pipeline_persist_and_query() {
    let store = CatchMeStore::new(":memory:").expect("in-memory store");
    let org = TreeOrganizer::new(OrganizerConfig::default());

    let events = vec![
        make_event(
            "test",
            EventType::Window {
                title: "main.rs — Code".into(),
                app: "VS Code".into(),
            },
            0,
        ),
        make_event(
            "test",
            EventType::Keyboard {
                text: "fn main() { println!(\"hello\"); }".into(),
            },
            2,
        ),
        make_event(
            "test",
            EventType::Mouse {
                x: 100.0,
                y: 200.0,
                click: true,
            },
            5,
        ),
        make_event(
            "test",
            EventType::Clipboard {
                content: "copied code snippet".into(),
            },
            8,
        ),
        make_event(
            "test",
            EventType::Window {
                title: "Google Chrome".into(),
                app: "Chrome".into(),
            },
            10,
        ),
        make_event(
            "test",
            EventType::Mouse {
                x: 500.0,
                y: 300.0,
                click: true,
            },
            12,
        ),
    ];

    // Persist raw events
    for e in &events {
        store.insert_event(e).expect("insert event");
    }

    // Organize into tree
    let tree = org.organize(&events);
    assert_eq!(tree.len(), 1, "should have 1 day node");
    assert_eq!(tree[0].level, TreeLevel::Day);
    assert_eq!(tree[0].event_count, 6);

    // Should have 1 session (all events are close together)
    let sessions = &tree[0].children;
    assert_eq!(sessions.len(), 1, "should have 1 session");
    assert_eq!(sessions[0].level, TreeLevel::Session);

    // Should have multiple app clusters
    let apps = &sessions[0].children;
    assert!(
        apps.len() >= 2,
        "expected >= 2 app clusters, got {}",
        apps.len()
    );
    assert!(apps.iter().all(|a| a.level == TreeLevel::App));

    // Persist tree
    org.persist_tree(&store, &tree).expect("persist tree");

    // Query back
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let queried = store.query_tree_for_day(&today).expect("query tree");
    assert!(!queried.is_empty(), "should find persisted nodes for today");
    // Should include Day, Session, and App nodes at minimum
    let levels: Vec<_> = queried.iter().map(|n| &n.level).collect();
    assert!(levels.contains(&&TreeLevel::Day));
    assert!(levels.contains(&&TreeLevel::Session));
}

/// Test multi-session clustering (gap > threshold → new session).
#[test]
fn test_multi_session_clustering() {
    let org = TreeOrganizer::new(OrganizerConfig {
        session_gap_secs: 30, // 30s gap → new session
        ..Default::default()
    });

    let events = vec![
        // Session 1
        make_event(
            "test",
            EventType::Keyboard {
                text: "session 1".into(),
            },
            0,
        ),
        make_event(
            "test",
            EventType::Keyboard {
                text: "still s1".into(),
            },
            10,
        ),
        // 60 second gap → session 2
        make_event(
            "test",
            EventType::Keyboard {
                text: "session 2".into(),
            },
            70,
        ),
        make_event(
            "test",
            EventType::Keyboard {
                text: "still s2".into(),
            },
            80,
        ),
        // 60 second gap → session 3
        make_event(
            "test",
            EventType::Keyboard {
                text: "session 3".into(),
            },
            140,
        ),
    ];

    let tree = org.organize(&events);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 3, "expected 3 sessions");
}

/// Event insertion and range query.
#[test]
fn test_events_in_range() {
    let store = CatchMeStore::new(":memory:").expect("in-memory store");
    let now = Utc::now();

    let events = vec![
        CatchMeEvent {
            id: "e1".into(),
            timestamp: now - Duration::hours(2),
            source: "test".into(),
            event_type: EventType::Keyboard {
                text: "early".into(),
            },
        },
        CatchMeEvent {
            id: "e2".into(),
            timestamp: now - Duration::hours(1),
            source: "test".into(),
            event_type: EventType::Keyboard {
                text: "middle".into(),
            },
        },
        CatchMeEvent {
            id: "e3".into(),
            timestamp: now,
            source: "test".into(),
            event_type: EventType::Keyboard {
                text: "recent".into(),
            },
        },
    ];

    for e in &events {
        store.insert_event(e).expect("insert");
    }

    // Query last 90 minutes — should get e2 and e3
    let start = now - Duration::minutes(90);
    let end = now + Duration::minutes(1);
    let result = store.events_in_range(&start, &end).expect("query");
    assert_eq!(result.len(), 2, "expected 2 events in range");
}

/// Notification and screenshot events are handled gracefully.
#[test]
fn test_notification_and_screenshot_events() {
    let org = TreeOrganizer::new(OrganizerConfig::default());

    let events = vec![
        make_event(
            "test",
            EventType::Notification {
                app: "Zalo".into(),
                title: "New message".into(),
                body: "Hello from customer".into(),
            },
            0,
        ),
        make_event(
            "test",
            EventType::Screenshot {
                path: "/tmp/screenshot.png".into(),
            },
            5,
        ),
    ];

    let tree = org.organize(&events);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].event_count, 2);
}
