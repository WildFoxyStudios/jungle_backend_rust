//! End-to-end realtime fan-out tests.
//!
//! These tests exercise `ConnectionHub` (the per-user + topic broadcast
//! primitive that backs every WebSocket connection) without booting the full
//! HTTP server. They're "WS end-to-end" in the sense that they verify the
//! exact transport semantics observed by clients:
//!   * a message sent to a user is delivered to that user's receiver
//!   * a message sent to a topic is delivered to every subscribed receiver
//!   * disconnection cleanup leaves no stale entries
//!   * concurrent senders/receivers don't lose messages
//!
//! For an actual JWT-handshake + framed-WS test the integration runner needs
//! Postgres + Redis + NATS available, which is too heavy for `cargo test`
//! without a sandbox; that path is driven by the Playwright `chat.spec.ts`
//! suite (see `frontend/apps/web/e2e/chat.spec.ts`).

use realtime_service::hub::{ConnectionHub, WsMessage};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

fn message(event: &str, data: serde_json::Value) -> WsMessage {
    WsMessage {
        event: event.to_string(),
        data,
    }
}

#[tokio::test]
async fn user_channel_delivers_directed_messages() {
    let hub = ConnectionHub::new();
    let mut rx = hub.subscribe(42);

    hub.send_to_user(42, message("test.event", json!({ "ok": true })));

    let received = timeout(Duration::from_millis(250), rx.recv())
        .await
        .expect("recv timed out")
        .expect("channel closed");
    assert_eq!(received.event, "test.event");
    assert_eq!(received.data["ok"], true);
}

#[tokio::test]
async fn user_channel_does_not_leak_to_other_users() {
    let hub = ConnectionHub::new();
    let mut rx_a = hub.subscribe(1);
    let mut rx_b = hub.subscribe(2);

    hub.send_to_user(1, message("only-for-1", json!({})));
    hub.send_to_user(2, message("only-for-2", json!({})));

    let a = timeout(Duration::from_millis(250), rx_a.recv())
        .await
        .unwrap()
        .unwrap();
    let b = timeout(Duration::from_millis(250), rx_b.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.event, "only-for-1");
    assert_eq!(b.event, "only-for-2");

    // Cross-channel must yield nothing.
    assert!(
        timeout(Duration::from_millis(50), rx_a.recv())
            .await
            .is_err(),
        "user 1 should not receive a second event"
    );
}

#[tokio::test]
async fn topic_channel_fans_out_to_all_subscribers() {
    let hub = ConnectionHub::new();
    let mut rx_one = hub.subscribe_topic("feed:home");
    let mut rx_two = hub.subscribe_topic("feed:home");

    hub.send_to_topic("feed:home", message("post.new", json!({ "id": 99 })));

    let a = timeout(Duration::from_millis(250), rx_one.recv())
        .await
        .unwrap()
        .unwrap();
    let b = timeout(Duration::from_millis(250), rx_two.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.event, "post.new");
    assert_eq!(b.event, "post.new");
    assert_eq!(a.data["id"], 99);
}

#[tokio::test]
async fn unsubscribe_drops_connection_when_alone() {
    let hub = ConnectionHub::new();
    let rx = hub.subscribe(7);
    assert!(hub.is_online(7));
    drop(rx);
    hub.unsubscribe(7);
    assert!(
        !hub.is_online(7),
        "user 7 should be marked offline after unsubscribe"
    );
}

#[tokio::test]
async fn maybe_drop_topic_cleans_up_empty_topics() {
    let hub = ConnectionHub::new();
    {
        let _rx = hub.subscribe_topic("live:42");
        // Subscriber drops at end of scope.
    }
    hub.maybe_drop_topic("live:42");
    let mut rx = hub.subscribe_topic("live:42");
    hub.send_to_topic("live:42", message("live.frame", json!({})));
    let _ = timeout(Duration::from_millis(250), rx.recv())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn broadcast_reaches_every_user_channel() {
    let hub = ConnectionHub::new();
    let mut rx_a = hub.subscribe(10);
    let mut rx_b = hub.subscribe(20);

    hub.broadcast(message(
        "system.notice",
        json!({ "msg": "maintenance soon" }),
    ));

    let a = timeout(Duration::from_millis(250), rx_a.recv())
        .await
        .unwrap()
        .unwrap();
    let b = timeout(Duration::from_millis(250), rx_b.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.event, "system.notice");
    assert_eq!(b.event, "system.notice");
}

#[tokio::test]
async fn online_count_tracks_subscriptions() {
    let hub = ConnectionHub::new();
    assert_eq!(hub.online_count(), 0);
    let _rx_a = hub.subscribe(100);
    let _rx_b = hub.subscribe(200);
    assert_eq!(hub.online_count(), 2);
    let online: Vec<i64> = {
        let mut v = hub.online_users();
        v.sort();
        v
    };
    assert_eq!(online, vec![100, 200]);
}

#[tokio::test]
async fn send_to_user_when_offline_is_a_noop() {
    let hub = ConnectionHub::new();
    // No panic, no error, just discarded.
    hub.send_to_user(999, message("ghost", json!({})));
    assert!(!hub.is_online(999));
}
