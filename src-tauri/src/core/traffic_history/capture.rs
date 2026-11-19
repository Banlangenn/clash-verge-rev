use crate::core::handle::Handle;
use anyhow::Result;
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use tauri_plugin_mihomo::models::Connections;

#[derive(Deserialize)]
pub(super) struct ClosedTraffic {
    pub id: String,
    pub process: String,
    pub destination: String,
    pub node: String,
    pub upload: u64,
    pub download: u64,
    pub increment: bool,
    #[serde(default, rename = "recordedAt")]
    pub recorded_at: i64,
}

#[derive(Deserialize)]
pub(super) struct TrafficCursor {
    pub session: String,
    pub cursor: u64,
}

#[derive(Deserialize)]
struct HistoryResponse {
    snapshot: Connections,
    #[serde(flatten)]
    cursor: TrafficCursor,
    closed: Vec<ClosedTraffic>,
    #[serde(default)]
    dropped: u64,
}

pub(super) struct Capture {
    pub connections: Connections,
    pub cursor: Option<TrafficCursor>,
    pub closed: Vec<ClosedTraffic>,
    pub dropped: u64,
}

pub(super) async fn read(cursor: Option<&TrafficCursor>) -> Result<Capture> {
    let mihomo = Handle::mihomo();
    let mut request = mihomo.load_ctx().build_request(Method::GET, "/connections/history")?;
    if let Some(cursor) = cursor {
        request = request.query(&[
            ("session", cursor.session.as_str()),
            ("after", &cursor.cursor.to_string()),
        ]);
    }
    let response = request.send().await?;
    if matches!(
        response.status(),
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        return Ok(Capture {
            connections: mihomo.get_connections().await?,
            cursor: None,
            closed: Vec::new(),
            dropped: 0,
        });
    }
    let history: HistoryResponse = response.error_for_status()?.json().await?;
    let dropped = cursor
        .filter(|previous| previous.session == history.cursor.session)
        .map_or(0, |previous| {
            history
                .cursor
                .cursor
                .saturating_sub(previous.cursor)
                .saturating_sub(history.closed.len() as u64)
        });
    Ok(Capture {
        connections: history.snapshot,
        cursor: Some(history.cursor),
        closed: history.closed,
        dropped: history.dropped.max(dropped),
    })
}

pub(super) fn should_continue(previous: Option<&TrafficCursor>, next: Option<&TrafficCursor>) -> bool {
    match (previous, next) {
        (Some(previous), Some(next)) => previous.session == next.session && previous.cursor <= next.cursor,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryResponse, TrafficCursor, should_continue};

    #[test]
    fn deserializes_active_connections_cursor_and_closure_records() -> serde_json::Result<()> {
        let capture = serde_json::from_value::<HistoryResponse>(serde_json::json!({
            "snapshot": {
                "uploadTotal": 500,
                "downloadTotal": 700,
                "connections": [],
            },
            "session": "core-a",
            "cursor": 4,
            "closed": [{
                "id": "connection-a",
                "process": "browser.exe",
                "node": "Node A",
                "destination": "example.test",
                "upload": 5,
                "download": 10,
                "increment": false,
            }],
        }))?;
        assert_eq!(capture.cursor.session, "core-a");
        assert_eq!(capture.cursor.cursor, 4);
        assert_eq!(capture.snapshot.download_total, 700);
        assert_eq!(capture.closed[0].destination, "example.test");
        Ok(())
    }

    #[test]
    fn discards_the_baseline_on_core_change_or_history_loss() {
        let original = TrafficCursor {
            session: "core-a".to_owned(),
            cursor: 5,
        };
        let same_core = TrafficCursor {
            session: "core-a".to_owned(),
            cursor: 10,
        };
        let new_core = TrafficCursor {
            session: "core-b".to_owned(),
            cursor: 10,
        };
        assert!(should_continue(Some(&original), Some(&same_core)));
        assert!(!should_continue(Some(&original), Some(&new_core)));
        assert!(!should_continue(Some(&same_core), Some(&original)));
        assert!(!should_continue(Some(&original), None));
        assert!(!should_continue(None, Some(&original)));
        assert!(should_continue(None, None));
    }
}
