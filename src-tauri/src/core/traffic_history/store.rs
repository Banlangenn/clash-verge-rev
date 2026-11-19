use super::{
    Bucket, TrafficAmount, TrafficBreakdown, TrafficBuckets, TrafficDimension, TrafficPoint, TrafficRoute,
    TrafficSummary, floor_minute,
};
#[cfg(any(windows, test))]
use anyhow::ensure;
use anyhow::{Context as _, Result};
use chrono::Duration;
#[cfg(any(windows, test))]
use chrono::{DateTime, NaiveDate};
use rusqlite::{Connection, Transaction, params};
#[cfg(any(windows, test))]
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};
#[cfg(any(windows, test))]
use std::{fs, io};

#[cfg(any(windows, test))]
const LEGACY_COLUMNS: [&str; 9] = [
    "minute",
    "scope",
    "process",
    "process_source",
    "destination",
    "node",
    "protocol",
    "upload_bytes",
    "download_bytes",
];

pub(super) fn open(path: &Path) -> Result<Connection> {
    let database = Connection::open(path).context("open traffic history database")?;
    database.busy_timeout(std::time::Duration::from_secs(3))?;
    database.execute_batch(
        "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS traffic (
            minute INTEGER NOT NULL,
            scope TEXT NOT NULL,
            process TEXT NOT NULL,
            destination TEXT NOT NULL,
            node TEXT NOT NULL,
            upload INTEGER NOT NULL,
            download INTEGER NOT NULL,
            PRIMARY KEY (minute, scope, process, destination, node)
         ) WITHOUT ROWID;
         CREATE INDEX IF NOT EXISTS traffic_core_minutes ON traffic (minute) WHERE scope = 'core';
         CREATE TABLE IF NOT EXISTS traffic_migrations (name TEXT PRIMARY KEY) WITHOUT ROWID;
         CREATE TABLE IF NOT EXISTS traffic_capture (
            minute INTEGER PRIMARY KEY,
            sampled_at INTEGER NOT NULL,
            interrupted INTEGER NOT NULL
         );",
    )?;
    Ok(database)
}

fn add(transaction: &Transaction<'_>, minute: i64, bucket: &Bucket, amount: TrafficAmount) -> Result<()> {
    transaction
        .prepare_cached(
            "INSERT INTO traffic (minute, scope, process, destination, node, upload, download)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT DO UPDATE SET upload = upload + excluded.upload, download = download + excluded.download",
        )?
        .execute(params![
            minute,
            bucket.scope,
            bucket.process,
            bucket.destination,
            bucket.node,
            amount.upload,
            amount.download
        ])?;
    Ok(())
}

pub(super) fn save_sample(
    database: &mut Connection,
    sampled_at: i64,
    buckets: &TrafficBuckets,
    backfill: BTreeMap<i64, TrafficBuckets>,
    cutoff: Option<i64>,
    interrupted: bool,
) -> Result<()> {
    let minute = floor_minute(sampled_at);
    let transaction = database.transaction()?;
    if let Some(cutoff) = cutoff {
        transaction.execute("DELETE FROM traffic WHERE minute < ?1", [cutoff])?;
        transaction.execute("DELETE FROM traffic_capture WHERE minute < ?1", [cutoff])?;
    }
    for (event_minute, amounts) in backfill {
        for (bucket, amount) in amounts {
            add(&transaction, event_minute, &bucket, amount)?;
        }
    }
    for (bucket, amount) in buckets {
        add(&transaction, minute, bucket, *amount)?;
    }
    transaction.execute(
        "INSERT INTO traffic_capture (minute, sampled_at, interrupted) VALUES (?1, ?2, ?3)
         ON CONFLICT DO UPDATE SET
           sampled_at = max(sampled_at, excluded.sampled_at),
           interrupted = max(interrupted, excluded.interrupted)",
        params![minute, sampled_at, interrupted],
    )?;
    transaction.commit()?;
    Ok(())
}

#[cfg(any(windows, test))]
#[derive(Deserialize)]
struct LegacyRow {
    minute: String,
    scope: String,
    process: String,
    destination: String,
    node: String,
    upload_bytes: u64,
    download_bytes: u64,
}

#[cfg(any(windows, test))]
pub(super) fn migrate(database: &mut Connection, directory: &Path, before: i64, cutoff: i64) -> Result<()> {
    let transaction = database.transaction()?;
    if transaction.query_row(
        "SELECT EXISTS (SELECT 1 FROM traffic_migrations WHERE name = 'legacy-csv')",
        [],
        |row| row.get::<_, bool>(0),
    )? {
        return Ok(());
    }

    let first_native_minute: Option<i64> =
        transaction.query_row("SELECT min(minute) FROM traffic WHERE scope = 'core'", [], |row| {
            row.get(0)
        })?;
    let before = before.min(first_native_minute.unwrap_or(before));

    let files = match fs::read_dir(directory) {
        Ok(files) => files,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    for file in files {
        let file = file?;
        let path = file.path();
        let name = file.file_name();
        let Some(date) = name
            .to_str()
            .and_then(|name| name.strip_prefix("traffic-"))
            .and_then(|name| name.strip_suffix(".csv"))
        else {
            continue;
        };
        if !file.file_type()?.is_file() || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            continue;
        }

        let mut reader =
            csv::Reader::from_path(&path).with_context(|| format!("open legacy traffic file {}", path.display()))?;
        ensure!(
            reader.headers()?.iter().eq(LEGACY_COLUMNS),
            "unrecognized legacy traffic columns in {}",
            path.display()
        );
        for record in reader.deserialize::<LegacyRow>() {
            let record = record.with_context(|| format!("read legacy traffic file {}", path.display()))?;
            let minute = DateTime::parse_from_str(&record.minute, "%Y-%m-%dT%H:%M%:z")?.timestamp();
            if minute < cutoff || minute >= before {
                continue;
            }
            let scope = match record.scope.as_str() {
                "core" => "core",
                "proxy" => "proxy",
                "unattributed" => "unattributed",
                other => anyhow::bail!("unrecognized legacy traffic scope: {other}"),
            };
            let bucket = Bucket {
                scope,
                process: super::normalize_process(&record.process),
                destination: record.destination,
                node: record.node,
            };
            add(
                &transaction,
                minute,
                &bucket,
                TrafficAmount {
                    upload: record.upload_bytes,
                    download: record.download_bytes,
                },
            )?;
        }
    }

    transaction.execute("INSERT INTO traffic_migrations VALUES ('legacy-csv')", [])?;
    transaction.commit()?;
    Ok(())
}

const BYTES_PER_GIBIBYTE: f64 = (1024_u64 * 1024 * 1024) as f64;

fn node_unit_price(node: &str) -> Option<f64> {
    let (_, suffix) = node.rsplit_once('|')?;
    let price = suffix.trim();
    if !price.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.') {
        return None;
    }
    price.parse::<f64>().ok().filter(|price| price.is_finite())
}

fn estimate_cost(scope: &str, node: &str, upload: u64, download: u64) -> Option<f64> {
    if scope != "proxy" {
        return None;
    }
    node_unit_price(node).map(|price| ((upload as f64 + download as f64) / BYTES_PER_GIBIBYTE) * price)
}

pub(super) fn summary(
    database: &Connection,
    from: i64,
    until: i64,
    route: TrafficRoute,
    dimension: TrafficDimension,
    process: Option<&str>,
) -> Result<TrafficSummary> {
    let transaction = database.unchecked_transaction()?;
    let scope = match route {
        TrafficRoute::Proxy => "proxy",
        TrafficRoute::Direct => "direct",
    };
    let column = match dimension {
        TrafficDimension::Application => "process",
        TrafficDimension::Destination => "destination",
        TrafficDimension::Node => "node",
    };
    let bounds: (Option<i64>, Option<i64>) = transaction.query_row(
        "SELECT min(minute), max(minute) FROM traffic WHERE scope = 'core'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let (first_capture, last_sample_at): (Option<i64>, Option<i64>) =
        transaction.query_row("SELECT min(minute), max(sampled_at) FROM traffic_capture", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
    let mut capture_query = transaction.prepare(
        "SELECT minute, interrupted FROM traffic_capture WHERE minute >= ?1 AND minute < ?2
         UNION ALL
         SELECT minute, 0 FROM traffic
          WHERE scope = 'core' AND minute >= ?1 AND minute < ?2 AND minute < ?3
         ORDER BY minute",
    )?;
    let mut capture_minutes = Vec::new();
    let mut interrupted_minutes = Vec::new();
    let mut capture_rows = capture_query.query(params![from, until, first_capture.unwrap_or(i64::MAX)])?;
    while let Some(row) = capture_rows.next()? {
        let minute = row.get(0)?;
        capture_minutes.push(minute);
        if row.get::<_, bool>(1)? {
            interrupted_minutes.push(minute);
        }
    }
    let (core_amount, unattributed) = transaction.query_row(
        "SELECT coalesce(sum(CASE WHEN scope = 'core' THEN upload END), 0),
                coalesce(sum(CASE WHEN scope = 'core' THEN download END), 0),
                coalesce(sum(CASE WHEN scope = 'unattributed' THEN upload END), 0),
                coalesce(sum(CASE WHEN scope = 'unattributed' THEN download END), 0)
         FROM traffic WHERE minute >= ?1 AND minute < ?2 AND scope IN ('core', 'unattributed')",
        params![from, until],
        |row| {
            Ok((
                TrafficAmount {
                    upload: row.get(0)?,
                    download: row.get(1)?,
                },
                TrafficAmount {
                    upload: row.get(2)?,
                    download: row.get(3)?,
                },
            ))
        },
    )?;

    let bindings = params![from, until, scope, process];
    let range = "FROM traffic WHERE minute >= ?1 AND minute < ?2 AND scope = ?3 AND (?4 IS NULL OR process = ?4)";
    let unknown_application = transaction.query_row(
        &format!("SELECT coalesce(sum(upload), 0), coalesce(sum(download), 0) {range} AND process = ''"),
        bindings,
        |row| {
            Ok(TrafficAmount {
                upload: row.get(0)?,
                download: row.get(1)?,
            })
        },
    )?;
    let mut points_query = transaction.prepare(&format!(
        "SELECT minute, node, sum(upload), sum(download) {range} GROUP BY minute, node ORDER BY minute"
    ))?;
    let mut point_rows = points_query.query(bindings)?;
    let mut points: Vec<TrafficPoint> = Vec::new();
    while let Some(row) = point_rows.next()? {
        let minute = row.get(0)?;
        let upload: u64 = row.get(2)?;
        let download: u64 = row.get(3)?;
        let estimated_cost = estimate_cost(scope, row.get_ref(1)?.as_str()?, upload, download);
        if let Some(point) = points.last_mut().filter(|point| point.minute == minute) {
            point.upload += upload;
            point.download += download;
            if let Some(cost) = estimated_cost {
                *point.estimated_cost.get_or_insert(0.0) += cost;
            }
        } else {
            points.push(TrafficPoint {
                minute,
                upload,
                download,
                estimated_cost,
            });
        }
    }
    let amount = points.iter().fold(TrafficAmount::default(), |mut total, point| {
        total.upload += point.upload;
        total.download += point.download;
        total
    });
    let estimated_cost = points
        .iter()
        .filter_map(|point| point.estimated_cost)
        .reduce(|a, b| a + b);
    let mut breakdown_query = transaction.prepare(&format!(
        "SELECT {column}, node, sum(upload), sum(download) {range} GROUP BY {column}, node"
    ))?;
    let mut breakdown_rows = breakdown_query.query(bindings)?;
    let mut groups = HashMap::<String, TrafficBreakdown>::new();
    while let Some(row) = breakdown_rows.next()? {
        let name: String = row.get(0)?;
        let group = groups.entry(name.clone()).or_insert_with(|| TrafficBreakdown {
            name,
            upload: 0,
            download: 0,
            estimated_cost: None,
            partial_cost: false,
        });
        let upload: u64 = row.get(2)?;
        let download: u64 = row.get(3)?;
        group.upload += upload;
        group.download += download;
        if let Some(cost) = estimate_cost(scope, row.get_ref(1)?.as_str()?, upload, download) {
            *group.estimated_cost.get_or_insert(0.0) += cost;
        } else if upload > 0 || download > 0 {
            group.partial_cost = true;
        }
    }
    let mut breakdown: Vec<_> = groups.into_values().collect();
    for group in &mut breakdown {
        group.partial_cost &= group.estimated_cost.is_some();
    }
    let mut cost_breakdown: Vec<_> = breakdown
        .iter()
        .filter(|row| row.estimated_cost.is_some())
        .cloned()
        .collect();
    cost_breakdown.sort_unstable_by(|left, right| {
        right
            .estimated_cost
            .unwrap_or_default()
            .total_cmp(&left.estimated_cost.unwrap_or_default())
            .then_with(|| left.name.cmp(&right.name))
    });
    cost_breakdown.truncate(100);
    breakdown.sort_unstable_by(|left, right| {
        (right.upload + right.download)
            .cmp(&(left.upload + left.download))
            .then_with(|| left.name.cmp(&right.name))
    });
    breakdown.truncate(100);

    Ok(TrafficSummary {
        first_minute: bounds.0.into_iter().chain(first_capture).min(),
        last_minute: bounds.1.into_iter().chain(last_sample_at.map(floor_minute)).max(),
        sampled_minutes: capture_minutes.len() as u64,
        capture_minutes,
        interrupted_minutes,
        last_sample_at: last_sample_at.or(bounds.1),
        core_amount,
        unattributed,
        unknown_application,
        amount,
        estimated_cost,
        points,
        breakdown,
        cost_breakdown,
    })
}

pub(super) const fn retention_cutoff(minute: i64) -> i64 {
    minute - Duration::days(7).num_seconds() + 60
}

#[cfg(test)]
mod tests {
    use super::{migrate, open, save_sample, summary};
    use crate::core::traffic_history::{Bucket, TrafficAmount, TrafficDimension, TrafficRoute};
    use std::{collections::HashMap, fs};

    fn save(
        database: &mut rusqlite::Connection,
        minute: i64,
        buckets: &HashMap<Bucket, TrafficAmount>,
        cutoff: Option<i64>,
    ) -> anyhow::Result<()> {
        save_sample(database, minute, buckets, Default::default(), cutoff, false)
    }

    #[test]
    fn estimates_only_priced_proxy_traffic_in_the_requested_range() -> anyhow::Result<()> {
        let mut database = open(std::path::Path::new(":memory:"))?;
        let gibibyte = 1024_u64.pow(3);

        let bucket = |scope, process: &str, node: &str| Bucket {
            scope,
            process: process.to_owned(),
            node: node.to_owned(),
            ..Bucket::default()
        };
        let traffic = TrafficAmount {
            upload: gibibyte / 2,
            download: gibibyte / 2,
        };
        let priced = HashMap::from([
            (bucket("proxy", "browser.exe", "● Example JP | 0.35"), traffic),
            (bucket("proxy", "updates.exe", "Example HK | 0.2"), traffic),
            (bucket("proxy", "browser.exe", "No price"), traffic),
            (bucket("direct", "browser.exe", "DIRECT | 0.35"), traffic),
        ]);
        save(&mut database, 60, &priced, None)?;
        let unpriced = HashMap::from([(bucket("proxy", "browser.exe", "No price | UK"), traffic)]);
        save(&mut database, 120, &unpriced, None)?;
        let free = HashMap::from([(bucket("proxy", "browser.exe", "Free | 0"), traffic)]);
        save(&mut database, 180, &free, None)?;

        let result = summary(&database, 60, 240, TrafficRoute::Proxy, TrafficDimension::Node, None)?;
        assert_eq!(result.amount.download, 5 * traffic.download);
        assert!(result.estimated_cost.is_some_and(|cost| (cost - 0.55).abs() < 1e-12));
        assert!(
            result.points[0]
                .estimated_cost
                .is_some_and(|cost| (cost - 0.55).abs() < 1e-12)
        );
        assert_eq!(result.points[1].estimated_cost, None);
        assert_eq!(result.points[2].estimated_cost, Some(0.0));
        let filtered = summary(
            &database,
            60,
            120,
            TrafficRoute::Proxy,
            TrafficDimension::Destination,
            Some("browser.exe"),
        )?;
        assert_eq!(filtered.estimated_cost, Some(0.35));
        assert_eq!(filtered.cost_breakdown[0].estimated_cost, Some(0.35));
        assert!(filtered.cost_breakdown[0].partial_cost);
        assert_eq!(result.cost_breakdown.len(), 3);
        assert_eq!(result.cost_breakdown[2].estimated_cost, Some(0.0));

        let direct = summary(
            &database,
            60,
            120,
            TrafficRoute::Direct,
            TrafficDimension::Application,
            None,
        )?;
        assert_eq!(direct.amount.download, traffic.download);
        assert_eq!(direct.estimated_cost, None);
        assert!(direct.cost_breakdown.is_empty());
        assert_eq!(direct.points[0].estimated_cost, None);
        Ok(())
    }

    #[test]
    fn legacy_import_is_idempotent_and_live_samples_keep_existing_history() -> anyhow::Result<()> {
        let directory = std::env::temp_dir().join(format!("verge-traffic-{}", nanoid::nanoid!()));
        fs::create_dir(&directory)?;
        scopeguard::defer! { let _ = fs::remove_dir_all(&directory); }
        let csv = directory.join("traffic-2026-11-18.csv");
        fs::write(
            csv,
            "\u{feff}minute,scope,process,process_source,destination,node,protocol,upload_bytes,download_bytes\r\n\
             2026-11-18T09:39+08:00,core,,,,,,1008,2048\r\n\
             2026-11-18T09:39+08:00,proxy,Code.exe,windows_tcp,example.com,\"node, one\",tcp,8,1024\r\n\
             2026-11-18T09:39+08:00,unattributed,,,,,,1000,1024\r\n",
        )?;
        let path = directory.join("history.sqlite3");
        let minute = chrono::DateTime::parse_from_rfc3339("2026-11-18T09:39:00+08:00")?.timestamp();
        let mut database = open(&path)?;
        migrate(&mut database, &directory, minute + 60, minute)?;
        drop(database);

        let mut database = open(&path)?;
        migrate(&mut database, &directory, minute + 60, minute)?;
        let result = summary(
            &database,
            minute,
            minute + 60,
            TrafficRoute::Proxy,
            TrafficDimension::Application,
            None,
        )?;
        assert_eq!(result.sampled_minutes, 1);
        assert_eq!(result.amount.download, 1024);
        assert_eq!(result.core_amount.download, 2048);
        assert_eq!(result.unattributed.download, 1024);
        let application = &result.breakdown[0].name;
        let detail = summary(
            &database,
            minute,
            minute + 60,
            TrafficRoute::Proxy,
            TrafficDimension::Node,
            Some(application),
        )?;
        assert_eq!(detail.breakdown[0].name, "node, one");

        let buckets = HashMap::from([(
            Bucket {
                scope: "proxy",
                process: application.clone(),
                destination: "example.com".to_owned(),
                node: "node, one".to_owned(),
            },
            TrafficAmount {
                upload: 2,
                download: 32,
            },
        )]);
        save(&mut database, minute, &buckets, Some(minute))?;
        let result = summary(
            &database,
            minute,
            minute + 60,
            TrafficRoute::Proxy,
            TrafficDimension::Application,
            None,
        )?;
        assert_eq!(result.amount.download, 1056);
        assert_eq!(result.breakdown.len(), 1);
        Ok(())
    }
}
