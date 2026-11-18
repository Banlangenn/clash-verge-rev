mod store;
#[cfg(windows)]
mod windows_process;

use crate::{Type, core::handle::Handle, logging, process::AsyncHandler, utils::dirs};
use anyhow::{Context as _, Result, ensure};
use chrono::Utc;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tauri_plugin_mihomo::models::{ConnectionMetaData, Connections};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
const SAMPLE_TIMEOUT: Duration = Duration::from_secs(3);

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

#[derive(Serialize, specta::Type)]
pub struct TrafficBreakdown {
    name: String,
    upload: u64,
    download: u64,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSummary {
    first_minute: Option<i64>,
    last_minute: Option<i64>,
    sampled_minutes: u64,
    core_amount: TrafficAmount,
    unattributed: TrafficAmount,
    amount: TrafficAmount,
    estimated_cost: Option<f64>,
    points: Vec<TrafficPoint>,
    breakdown: Vec<TrafficBreakdown>,
}

#[derive(Default, Eq, Hash, PartialEq)]
struct Bucket {
    scope: &'static str,
    process: String,
    destination: String,
    node: String,
}

struct ConnectionSample {
    amount: TrafficAmount,
    process: String,
}

#[derive(Default)]
struct Sample {
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
    #[cfg(windows)]
    socket_owners: Option<windows_process::SocketOwners>,
}

impl Sampler {
    fn measure(mut self, snapshot: Connections, previous: Option<&Sample>) -> (Sample, HashMap<Bucket, TrafficAmount>) {
        let mut current = Sample {
            amount: TrafficAmount {
                upload: snapshot.upload_total,
                download: snapshot.download_total,
            },
            ..Sample::default()
        };
        let previous = previous.filter(|previous| {
            previous.amount.upload <= current.amount.upload && previous.amount.download <= current.amount.download
        });
        let mut buckets = HashMap::<Bucket, TrafficAmount>::new();
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
            let delta = current.amount.delta(previous.amount);
            buckets.insert(
                Bucket {
                    scope: "core",
                    ..Bucket::default()
                },
                delta,
            );
            buckets.insert(
                Bucket {
                    scope: "unattributed",
                    ..Bucket::default()
                },
                delta.delta(observed),
            );
        }

        (current, buckets)
    }

    fn socket_process(&mut self, metadata: &ConnectionMetaData) -> String {
        #[cfg(windows)]
        if matches!(metadata.network, tauri_plugin_mihomo::models::Network::TCP) {
            return self
                .socket_owners
                .get_or_insert_with(windows_process::SocketOwners::read)
                .resolve(metadata);
        }
        let _ = metadata;
        String::new()
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
    let now = Utc::now().timestamp();
    now - now.rem_euclid(60)
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
        let mut previous = None;
        let mut pruned_day = None;
        loop {
            interval.tick().await;
            if Handle::global().is_exiting() {
                previous = None;
                continue;
            }

            let Ok(Ok(snapshot)) = tokio::time::timeout(SAMPLE_TIMEOUT, Handle::mihomo().get_connections()).await
            else {
                previous = None;
                continue;
            };
            let minute = current_minute();
            let day = minute.div_euclid(86_400);
            let cutoff = (pruned_day != Some(day)).then(|| store::retention_cutoff(minute));
            let finished = AsyncHandler::spawn_blocking(move || {
                let (sample, buckets) = Sampler::default().measure(snapshot, previous.as_ref());
                if !buckets.is_empty() {
                    let mut database = store::open(&database_path()?)?;
                    store::save(&mut database, minute, &buckets, cutoff)?;
                }
                Ok::<_, anyhow::Error>((sample, !buckets.is_empty()))
            })
            .await;
            match finished {
                Ok(Ok((sample, saved))) => {
                    previous = Some(sample);
                    if saved {
                        pruned_day = Some(day);
                    }
                }
                Ok(Err(error)) => {
                    logging!(warn, Type::File, "Traffic history sample could not be saved: {error:#}");
                    previous = None;
                }
                Err(error) => {
                    logging!(warn, Type::File, "Traffic history sample task failed: {error:#}");
                    previous = None;
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
    use super::{Bucket, Sampler};
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
        let (previous, baseline) = Sampler::default().measure(snapshot(100, 20, 30), None);
        assert!(baseline.is_empty());
        let (current, buckets) = Sampler::default().measure(snapshot(150, 30, 55), Some(&previous));
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
        let (_, after_reset) = Sampler::default().measure(snapshot(25, 12, 8), Some(&current));
        assert!(after_reset.is_empty());
    }
}
