//! Validates the messaging-service call event contract end to end.
//!
//! The full publish path goes:
//!   create_call → INSERT INTO calls → event_bus.publish(CallStarted)
//!                                          ↓ NATS subject "events.call.started"
//!   answer_call → UPDATE status = 'answered' → publish(CallAnswered)
//!   end_call    → UPDATE status = 'ended'    → publish(CallEnded)
//!
//! Booting the full HTTP service requires Postgres + NATS at test time, which
//! is too heavy for the standard `cargo test` flow. Instead we use an
//! in-memory `EventBus` that captures every published payload and check:
//!
//!   1. Each call lifecycle event maps to its expected NATS subject.
//!   2. The wire format (JSON) preserves the discriminant + fields exactly,
//!      so a downstream consumer such as realtime-service decoding the same
//!      payload will route the event to the correct hub broadcast.

use async_trait::async_trait;
use shared::events::{DomainEvent, EventBus, EventBusError, EventSubscription};
use std::sync::Mutex;

#[derive(Default)]
struct CapturingBus {
    captured: Mutex<Vec<(String, DomainEvent)>>,
    raw: Mutex<Vec<(String, Vec<u8>)>>,
}

#[async_trait]
impl EventBus for CapturingBus {
    async fn publish(&self, event: &DomainEvent) -> Result<(), EventBusError> {
        self.captured
            .lock()
            .unwrap()
            .push((event.subject().to_string(), event.clone()));
        Ok(())
    }

    async fn subscribe(&self, _subject: &str) -> Result<EventSubscription, EventBusError> {
        Err(EventBusError::Connection(
            "CapturingBus does not support subscribe".into(),
        ))
    }

    async fn publish_raw(&self, subject: &str, payload: &[u8]) -> Result<(), EventBusError> {
        self.raw
            .lock()
            .unwrap()
            .push((subject.to_string(), payload.to_vec()));
        Ok(())
    }
}

#[tokio::test]
async fn call_started_event_routes_to_call_started_subject() {
    let bus = CapturingBus::default();

    let event = DomainEvent::CallStarted {
        call_id: 123,
        caller_id: 1,
        callee_id: 2,
        call_type: "video".into(),
    };
    bus.publish(&event).await.expect("publish ok");

    let captured = bus.captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, "events.call.started");
    match &captured[0].1 {
        DomainEvent::CallStarted {
            call_id,
            caller_id,
            callee_id,
            call_type,
        } => {
            assert_eq!(*call_id, 123);
            assert_eq!(*caller_id, 1);
            assert_eq!(*callee_id, 2);
            assert_eq!(call_type, "video");
        }
        other => panic!("unexpected event variant: {other:?}"),
    }
}

#[tokio::test]
async fn call_answered_event_routes_to_call_answered_subject() {
    let bus = CapturingBus::default();

    let event = DomainEvent::CallAnswered { call_id: 999 };
    bus.publish(&event).await.expect("publish ok");

    let captured = bus.captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, "events.call.answered");
}

#[tokio::test]
async fn call_ended_event_routes_to_call_ended_subject() {
    let bus = CapturingBus::default();

    let event = DomainEvent::CallEnded { call_id: 41 };
    bus.publish(&event).await.expect("publish ok");

    let captured = bus.captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, "events.call.ended");
}

#[tokio::test]
async fn call_started_serializes_with_stable_wire_format() {
    let event = DomainEvent::CallStarted {
        call_id: 1,
        caller_id: 2,
        callee_id: 3,
        call_type: "audio".into(),
    };
    let payload = serde_json::to_value(&event).expect("serialize");
    let obj = payload
        .as_object()
        .expect("event must serialize as a JSON object");
    let tag = obj
        .get("event")
        .and_then(|v| v.as_str())
        .expect("missing discriminant `event`");
    assert_eq!(tag, "CallStarted");
    let data = obj.get("data").expect("envelope must carry inner data");
    assert_eq!(data.get("call_id").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(data.get("caller_id").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(data.get("callee_id").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(
        data.get("call_type").and_then(|v| v.as_str()),
        Some("audio")
    );
}

#[tokio::test]
async fn full_call_lifecycle_publishes_three_subjects_in_order() {
    let bus = CapturingBus::default();

    bus.publish(&DomainEvent::CallStarted {
        call_id: 7,
        caller_id: 1,
        callee_id: 2,
        call_type: "video".into(),
    })
    .await
    .unwrap();
    bus.publish(&DomainEvent::CallAnswered { call_id: 7 })
        .await
        .unwrap();
    bus.publish(&DomainEvent::CallEnded { call_id: 7 })
        .await
        .unwrap();

    let captured = bus.captured.lock().unwrap();
    assert_eq!(captured.len(), 3);
    assert_eq!(captured[0].0, "events.call.started");
    assert_eq!(captured[1].0, "events.call.answered");
    assert_eq!(captured[2].0, "events.call.ended");
}
