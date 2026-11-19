mod capture;
mod store;
#[cfg(windows)]
mod windows_process;

use crate::{Type, core::handle::Handle, logging, process::AsyncHandler, utils::dirs};
use anyhow::{Context as _, Result, ensure};
use capture::ClosedTraffic;
use chrono::Utc;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tauri_plugin_mihomo::models::{ConnectionMetaData, Connections};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(3);
const SAMPLE_TIMEOUT: Duration = Duration::from_secs(3);
const SAMPLE_GAP_SECONDS: i64 = 12;

#[derive(Clone, Copy, Default, Serialize, specta::Type)]
pub struct TrafficAmount {
    upload: u64,
    download: u64,
}

impl TrafficAmount {
    fn delta(self, previous: Self) -> Self {
        Self {
            upload: self.upload.saturating_sub(previous.upload),
            download: self.download.saturating_sub(previous.download),
        }
    }

    fn add(&mut self, amount: Self) {
        self.upload += amount.upload;
        self.download += amount.download;
    }

    fn is_empty(self) -> bool {
        self.upload == 0 && self.download == 0
    }
}

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrafficRoute {
    Proxy,
    Direct,
}

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrafficDimension {
    Application,
    Destination,
    Node,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPoint {
    minute: i64,
    upload: u64,
    download: u64,
    estimated_cost: Option<f64>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct TrafficBreakdown {
    name: String,
    upload: u64,
    download: u64,
    #[serde(rename = "estimatedCost")]
    estimated_cost: Option<f64>,
    #[serde(rename = "partialCost")]
    partial_cost: bool,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSummary {
    first_minute: Option<i64>,
    last_minute: Option<i64>,
    sampled_minutes: u64,
    capture_minutes: Vec<i64>,
    interrupted_minutes: Vec<i64>,
    last_sample_at: Option<i64>,
    core_amount: TrafficAmount,
    unattributed: TrafficAmount,
    unknown_application: TrafficAmount,
    amount: TrafficAmount,
    estimated_cost: Option<f64>,
    points: Vec<TrafficPoint>,
    breakdown: Vec<TrafficBreakdown>,
    cost_breakdown: Vec<TrafficBreakdown>,
}

#[derive(Default, Eq, Hash, PartialEq)]
struct Bucket {
    scope: &'static str,
    process: String,
    destination: String,
    node: String,
}

type TrafficBuckets = HashMap<Bucket, TrafficAmount>;

struct ConnectionSample {
    amount: TrafficAmount,
    process: String,
}

#[derive(Default)]
struct Sample {
    sampled_at: i64,
    amount: TrafficAmount,
    connections: HashMap<String, ConnectionSample>,
}

fn process_basename(process: &str) -> &str {
    process.rsplit(['/', '\\']).next().unwrap_or_default()
}

fn normalize_process(process: &str) -> String {
    let name = process_basename(process);
    if cfg!(windows) {
        name.to_ascii_lowercase()
    } else {
        name.to_owned()
    }
}

fn connection_process(metadata: &ConnectionMetaData, previous: Option<&ConnectionSample>) -> String {
    let reported = if metadata.process.is_empty() {
        &metadata.process_path
    } else {
        &metadata.process
    };
    if !reported.is_empty() {
        return normalize_process(reported);
    }
    previous.map(|previous| previous.process.clone()).unwrap_or_default()
}

#[derive(Default)]
struct Sampler {
    sampled_at: i64,
    #[cfg(windows)]
    socket_owners: windows_process::SocketOwners,
}

impl Sampler {
    fn measure(
        mut self,
        snapshot: Connections,
        previous: Option<&Sample>,
        closed: Vec<ClosedTraffic>,
    ) -> (Sample, TrafficBuckets, BTreeMap<i64, TrafficBuckets>) {
        let mut current = Sample {
            sampled_at: self.sampled_at,
            amount: TrafficAmount {
                upload: snapshot.upload_total,
                download: snapshot.download_total,
            },
            ..Sample::default()
        };
        let previous = previous.filter(|previous| {
            previous.amount.upload <= current.amount.upload && previous.amount.download <= current.amount.download
        });
        let mut buckets = TrafficBuckets::new();
        let mut backfill = BTreeMap::<i64, TrafficBuckets>::new();
        let mut closed_before_current_minute = Vec::new();
        let mut observed = TrafficAmount::default();

        for connection in snapshot.connections.into_iter().flatten() {
            let amount = TrafficAmount {
                upload: connection.upload,
                download: connection.download,
            };
            let prior = previous.and_then(|previous| previous.connections.get(&connection.id));
            let delta = amount.delta(prior.map(|sample| sample.amount).unwrap_or_default());
            let mut process = connection_process(&connection.metadata, prior);
            if previous.is_some()
                && !delta.is_empty()
                && let Some(node) = connection.chains.first()
                && let Some(scope) = traffic_scope(node)
            {
                if process.is_empty() {
                    process = self.socket_process(&connection.metadata);
                }
                let destination = [
                    connection.metadata.host,
                    connection.metadata.sniff_host,
                    connection.metadata.destination_ip,
                ]
                .into_iter()
                .find(|value| !value.is_empty())
                .unwrap_or_default();
                buckets
                    .entry(Bucket {
                        scope,
                        process: process.clone(),
                        destination,
                        node: node.clone(),
                    })
                    .or_default()
                    .add(delta);
                observed.add(delta);
            }
            current
                .connections
                .insert(connection.id, ConnectionSample { amount, process });
        }

        if let Some(previous) = previous {
            let sample_minute = floor_minute(self.sampled_at);
            let earliest_event_minute = store::retention_cutoff(sample_minute);
            for connection in closed {
                let Some(scope) = traffic_scope(&connection.node) else {
                    continue;
                };
                let prior = previous.connections.get(&connection.id);
                let amount = TrafficAmount {
                    upload: connection.upload,
                    download: connection.download,
                };
                let delta = if connection.increment {
                    amount
                } else {
                    amount.delta(prior.map(|sample| sample.amount).unwrap_or_default())
                };
                if delta.is_empty() {
                    continue;
                }
                let process = if connection.process.is_empty() {
                    prior.map(|sample| sample.process.clone()).unwrap_or_default()
                } else {
                    normalize_process(&connection.process)
                };
                let bucket = Bucket {
                    scope,
                    process,
                    destination: connection.destination,
                    node: connection.node,
                };
                let minute = floor_minute(connection.recorded_at);
                if connection.recorded_at > 0
                    && connection.recorded_at >= previous.sampled_at
                    && minute >= earliest_event_minute
                    && minute < sample_minute
                {
                    closed_before_current_minute.push((minute, bucket, delta));
                } else {
                    buckets.entry(bucket).or_default().add(delta);
                }
                observed.add(delta);
            }
            let delta = current.amount.delta(previous.amount);
            let mut current_core = delta;
            let reconciled = observed.upload <= delta.upload && observed.download <= delta.download;
            for (minute, bucket, amount) in closed_before_current_minute {
                if reconciled {
                    let historical = backfill.entry(minute).or_default();
                    historical.entry(bucket).or_default().add(amount);
                    historical
                        .entry(Bucket {
                            scope: "core",
                            ..Bucket::default()
                        })
                        .or_default()
                        .add(amount);
                    current_core = current_core.delta(amount);
                } else {
                    buckets.entry(bucket).or_default().add(amount);
                }
            }
            buckets.insert(
                Bucket {
                    scope: "core",
                    ..Bucket::default()
                },
                current_core,
            );
            buckets.insert(
                Bucket {
                    scope: "unattributed",
                    ..Bucket::default()
                },
                delta.delta(observed),
            );
        }

        (current, buckets, backfill)
    }

    fn socket_process(&mut self, metadata: &ConnectionMetaData) -> String {
        #[cfg(windows)]
        {
            self.socket_owners.resolve(metadata)
        }
        #[cfg(not(windows))]
        {
            let _ = metadata;
            String::new()
        }
    }
}

fn traffic_scope(node: &str) -> Option<&'static str> {
    match node.to_ascii_uppercase().as_str() {
        "DIRECT" | "PASS" | "COMPATIBLE" => Some("direct"),
        "REJECT" | "REJECT-DROP" | "" => None,
        _ => Some("proxy"),
    }
}

fn database_path() -> Result<PathBuf> {
    Ok(dirs::app_home_dir()?.join("traffic-history.sqlite3"))
}

fn current_minute() -> i64 {
    floor_minute(Utc::now().timestamp())
}

fn floor_minute(timestamp: i64) -> i64 {
    timestamp - timestamp.rem_euclid(60)
}

fn run_migration() -> Result<()> {
    #[cfg(windows)]
    if let Some(root) = ::dirs::data_local_dir() {
        let minute = current_minute();
        let mut database = store::open(&database_path()?)?;
        store::migrate(
            &mut database,
            &root.join("ClashTrafficLedger"),
            minute + 60,
            store::retention_cutoff(minute),
        )?;
    }
    Ok(())
}

pub(crate) fn start() {
    AsyncHandler::spawn(|| async {
        match AsyncHandler::spawn_blocking(run_migration).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => logging!(warn, Type::Setup, "Traffic history migration failed: {error:#}"),
            Err(error) => logging!(warn, Type::Setup, "Traffic history migration task failed: {error:#}"),
        }
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous: Option<Arc<Sample>> = None;
        let mut cursor = None;
        let mut last_saved_minute: Option<i64> = None;
        let mut capture_failed = false;
        loop {
            interval.tick().await;
            if Handle::global().is_exiting() {
                return;
            }

            let Ok(Ok(capture)) = tokio::time::timeout(SAMPLE_TIMEOUT, capture::read(cursor.as_ref())).await else {
                capture_failed = true;
                if cursor.is_none() {
                    previous = None;
                }
                continue;
            };
            let baseline = previous
                .as_ref()
                .filter(|_| capture::should_continue(cursor.as_ref(), capture.cursor.as_ref()))
                .cloned();
            let capture::Capture {
                connections,
                cursor: next_cursor,
                closed,
                dropped,
            } = capture;
            let sampled_at = Utc::now().timestamp();
            let minute = floor_minute(sampled_at);
            let interrupted = capture_failed
                || dropped > 0
                || previous.as_ref().is_some_and(|previous| {
                    baseline.is_none() || sampled_at.saturating_sub(previous.sampled_at) > SAMPLE_GAP_SECONDS
                });
            let day = minute.div_euclid(86_400);
            let cutoff = (last_saved_minute.map(|saved| saved.div_euclid(86_400)) != Some(day))
                .then(|| store::retention_cutoff(minute));
            let finished = AsyncHandler::spawn_blocking(move || {
                let (sample, buckets, backfill) = Sampler {
                    sampled_at,
                    ..Sampler::default()
                }
                .measure(connections, baseline.as_deref(), closed);
                let interrupted = interrupted || (baseline.is_some() && buckets.is_empty());
                let save = interrupted
                    || last_saved_minute != Some(minute)
                    || !backfill.is_empty()
                    || buckets.values().any(|amount| !amount.is_empty());
                if save {
                    let mut database = store::open(&database_path()?)?;
                    store::save_sample(&mut database, sampled_at, &buckets, backfill, cutoff, interrupted)?;
                }
                Ok::<_, anyhow::Error>((sample, save))
            })
            .await;
            match finished {
                Ok(Ok((sample, saved))) => {
                    previous = Some(Arc::new(sample));
                    cursor = next_cursor;
                    capture_failed = false;
                    if saved {
                        last_saved_minute = Some(minute);
                    }
                }
                Ok(Err(error)) => {
                    capture_failed = true;
                    logging!(warn, Type::File, "Traffic history sample could not be saved: {error:#}");
                    if cursor.is_none() {
                        previous = None;
                    }
                }
                Err(error) => {
                    capture_failed = true;
                    logging!(warn, Type::File, "Traffic history sample task failed: {error:#}");
                    if cursor.is_none() {
                        previous = None;
                    }
                }
            }
        }
    });
}

pub(crate) async fn read_summary(
    from: i64,
    until: i64,
    route: TrafficRoute,
    dimension: TrafficDimension,
    process: Option<String>,
) -> Result<TrafficSummary> {
    ensure!(
        from < until && until.saturating_sub(from) <= 8 * 86_400,
        "invalid traffic history time range"
    );
    AsyncHandler::spawn_blocking(move || {
        let database = store::open(&database_path()?)?;
        store::summary(&database, from, until, route, dimension, process.as_deref())
    })
    .await
    .context("traffic history query task failed")?
}

#[cfg(test)]
mod tests {
    use super::{Bucket, ClosedTraffic, Sampler, TrafficDimension, TrafficRoute, store};
    use tauri_plugin_mihomo::models::{Connection, ConnectionMetaData, Connections};

    fn snapshot(total: u64, proxy: u64, direct: u64) -> Connections {
        let connection = |id: &str, node: &str, download| Connection {
            id: id.to_owned(),
            download,
            chains: vec![node.to_owned()],
            metadata: ConnectionMetaData {
                process: "Example.exe".to_owned(),
                ..ConnectionMetaData::default()
            },
            ..Connection::default()
        };
        Connections {
            download_total: total,
            connections: Some(vec![
                connection("proxy", "Node", proxy),
                connection("direct", "DIRECT", direct),
            ]),
            ..Connections::default()
        }
    }

    #[test]
    fn sampler_distinguishes_route_deltas_and_does_not_count_core_reset_as_traffic() {
        let (previous, baseline, _) = Sampler::default().measure(snapshot(100, 20, 30), None, Vec::new());
        assert!(baseline.is_empty());
        let (current, buckets, _) = Sampler::default().measure(snapshot(150, 30, 55), Some(&previous), Vec::new());
        let download = |scope| {
            buckets
                .iter()
                .find(|(bucket, _)| bucket.scope == scope)
                .map(|(_, amount)| amount.download)
        };
        assert_eq!(download("core"), Some(50));
        assert_eq!(download("proxy"), Some(10));
        assert_eq!(download("direct"), Some(25));
        assert_eq!(
            buckets[&Bucket {
                scope: "unattributed",
                ..Bucket::default()
            }]
                .download,
            15
        );
        let (_, after_reset, _) = Sampler::default().measure(snapshot(25, 12, 8), Some(&current), Vec::new());
        assert!(after_reset.is_empty());
    }

    #[test]
    fn records_between_sample_closures_and_only_new_bytes_for_previously_active_connections() {
        let (previous, _, _) = Sampler::default().measure(snapshot(100, 20, 30), None, Vec::new());
        let closure = |id: &str, process: &str, download, increment| ClosedTraffic {
            id: id.to_owned(),
            process: process.to_owned(),
            destination: "example.test".to_owned(),
            node: "Node".to_owned(),
            upload: 0,
            download,
            increment,
            recorded_at: 0,
        };
        let (_, buckets, _) = Sampler::default().measure(
            Connections {
                download_total: 155,
                ..Connections::default()
            },
            Some(&previous),
            vec![
                closure("proxy", "", 30, false),
                closure("proxy", "", 5, true),
                closure("short", "short.exe", 40, false),
            ],
        );
        let bytes = |process: &str| {
            buckets
                .iter()
                .find(|(bucket, _)| bucket.scope == "proxy" && bucket.process.eq_ignore_ascii_case(process))
                .map(|(_, amount)| amount.download)
        };
        assert_eq!(bytes("example.exe"), Some(15));
        assert_eq!(bytes("short.exe"), Some(40));
        assert!(
            buckets[&Bucket {
                scope: "unattributed",
                ..Bucket::default()
            }]
                .is_empty()
        );
    }

    #[test]
    fn saves_recovered_closed_bytes_in_the_event_minute_without_marking_it_sampled() -> anyhow::Result<()> {
        let mut database = store::open(std::path::Path::new(":memory:"))?;
        let sample_at = |sampled_at| Sampler {
            sampled_at,
            ..Sampler::default()
        };
        let (baseline, starting_buckets, no_backfill) = sample_at(50).measure(snapshot(100, 20, 30), None, vec![]);
        store::save_sample(&mut database, 50, &starting_buckets, no_backfill, None, false)?;

        let (_, buckets, backfill) = sample_at(130).measure(
            Connections {
                download_total: 164,
                ..Connections::default()
            },
            Some(&baseline),
            vec![ClosedTraffic {
                id: "proxy".to_owned(),
                process: String::new(),
                destination: "example.test".to_owned(),
                node: "Node".to_owned(),
                upload: 0,
                download: 76,
                increment: false,
                recorded_at: 70,
            }],
        );
        store::save_sample(&mut database, 130, &buckets, backfill, None, true)?;
        store::save_sample(&mut database, 145, &Default::default(), Default::default(), None, false)?;

        let result = store::summary(
            &database,
            0,
            180,
            TrafficRoute::Proxy,
            TrafficDimension::Application,
            None,
        )?;
        assert_eq!(result.capture_minutes, [0, 120]);
        assert_eq!(result.interrupted_minutes, [120]);
        assert_eq!(result.last_sample_at, Some(145));
        assert_eq!(result.core_amount.download, 64);
        assert_eq!(result.amount.download, 56);
        assert_eq!(result.unattributed.download, 8);
        assert_eq!(result.points[0].minute, 60);
        assert_eq!(result.unknown_application.download, 0);
        Ok(())
    }
}
