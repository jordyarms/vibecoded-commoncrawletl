use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};

/// Run Phase 5: Score domains based on geo-filtered results.
pub fn run(
    workdir: &Path,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase5_complete {
        info!("Phase 5 already complete, skipping");
        return Ok(());
    }

    if !checkpoint.phase4_complete {
        return Err(Error::PhaseNotComplete { phase: 4 });
    }

    // Load all extracted events to get total counts per domain
    let extracted_dir = workdir.join("extracted");
    let mut domain_total: HashMap<String, u64> = HashMap::new();

    if extracted_dir.exists() {
        for entry in std::fs::read_dir(&extracted_dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "ndjson") {
                let file = std::fs::File::open(entry.path())?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line?;
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(domain) = val.get("domain").and_then(|d| d.as_str()) {
                            *domain_total.entry(domain.to_string()).or_default() += 1;
                        }
                    }
                }
            }
        }
    }

    // Load geo-filtered events
    let geofiltered_path = workdir.join("geofiltered_events.ndjson");
    if !geofiltered_path.exists() {
        return Err(Error::FileNotFound {
            path: geofiltered_path,
        });
    }

    let mut domain_stats: HashMap<String, DomainScore> = HashMap::new();

    let file = std::fs::File::open(&geofiltered_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
            let domain = val
                .get("domain")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let confidence = val
                .get("geo_confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0);
            let strategy = val
                .get("geo_strategy")
                .and_then(|s| s.as_str())
                .unwrap_or("none")
                .to_string();

            let stats = domain_stats.entry(domain).or_default();
            stats.gta_events += 1;
            stats.total_confidence += confidence;
            *stats.strategy_counts.entry(strategy).or_default() += 1;
        }
    }

    // Merge total counts and classify
    let mut writer = AtomicWriter::new(workdir.join("domain_scores.csv"))?;
    writeln!(
        writer,
        "domain,classification,total_events,gta_events,gta_ratio,avg_confidence,top_strategy"
    )?;

    let mut scored: Vec<(String, DomainScore, u64)> = domain_stats
        .into_iter()
        .map(|(domain, stats)| {
            let total = domain_total.get(&domain).copied().unwrap_or(0);
            (domain, stats, total)
        })
        .collect();

    scored.sort_by(|a, b| {
        b.1.gta_events
            .cmp(&a.1.gta_events)
            .then(b.2.cmp(&a.2))
    });

    for (domain, stats, total) in &scored {
        let total = *total;
        let gta_ratio = if total > 0 {
            stats.gta_events as f64 / total as f64
        } else {
            0.0
        };
        let avg_conf = if stats.gta_events > 0 {
            stats.total_confidence / stats.gta_events as f64
        } else {
            0.0
        };

        let top_strategy = stats
            .strategy_counts
            .iter()
            .max_by_key(|(_, v)| **v)
            .map(|(k, _)| k.as_str())
            .unwrap_or("none");

        let classification = classify(avg_conf, gta_ratio, stats.gta_events);

        writeln!(
            writer,
            "{},{},{},{},{:.3},{:.3},{}",
            domain, classification, total, stats.gta_events, gta_ratio, avg_conf, top_strategy
        )?;
    }

    writer.commit()?;

    info!("Phase 5 complete: scored {} domains", scored.len());

    checkpoint.phase5_complete = true;
    checkpoint.save(checkpoint_path)?;

    Ok(())
}

#[derive(Debug, Default)]
struct DomainScore {
    gta_events: u64,
    total_confidence: f64,
    strategy_counts: HashMap<String, u64>,
}

fn classify(avg_confidence: f64, gta_ratio: f64, gta_events: u64) -> &'static str {
    if avg_confidence >= 0.8 && gta_ratio >= 0.5 && gta_events >= 5 {
        "Confirmed"
    } else if avg_confidence >= 0.75 && gta_ratio >= 0.5 && gta_events >= 3 {
        "Likely"
    } else if avg_confidence >= 0.5 && gta_ratio >= 0.1 {
        "Possible"
    } else if gta_events > 0 {
        "Review"
    } else {
        "Exclude"
    }
}
