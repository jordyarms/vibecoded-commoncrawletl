use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};

/// Run Phase 6: Generate final output files.
pub fn run(
    workdir: &Path,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase6_complete {
        info!("Phase 6 already complete, skipping");
        return Ok(());
    }

    if !checkpoint.phase5_complete {
        return Err(Error::PhaseNotComplete { phase: 5 });
    }

    // Load domain scores
    let scores_path = workdir.join("domain_scores.csv");
    if !scores_path.exists() {
        return Err(Error::FileNotFound { path: scores_path });
    }

    let mut domain_classifications: HashMap<String, DomainRecord> = HashMap::new();
    let mut rdr = csv::Reader::from_path(&scores_path)?;

    for result in rdr.records() {
        let record = result?;
        if let Some(domain) = record.get(0) {
            let classification = record.get(1).unwrap_or("").to_string();
            let total_events = record.get(2).and_then(|s| s.parse().ok()).unwrap_or(0u64);
            let gta_events = record.get(3).and_then(|s| s.parse().ok()).unwrap_or(0u64);
            let gta_ratio = record.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0f64);
            let avg_confidence = record.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0f64);

            domain_classifications.insert(
                domain.to_string(),
                DomainRecord {
                    classification,
                    total_events,
                    gta_events,
                    gta_ratio,
                    avg_confidence,
                },
            );
        }
    }

    // 1. toronto_event_sources.csv — ranked domain list
    generate_sources_csv(workdir, &domain_classifications)?;

    // 2. toronto_event_samples.ndjson — top 3 events per INCLUDE domain
    generate_samples_ndjson(workdir, &domain_classifications)?;

    // 3. manual_review_queue.csv — UNKNOWN domains for human review
    generate_review_csv(workdir, &domain_classifications)?;

    info!("Phase 6 complete: generated output files");

    checkpoint.phase6_complete = true;
    checkpoint.save(checkpoint_path)?;

    Ok(())
}

#[derive(Debug)]
struct DomainRecord {
    classification: String,
    total_events: u64,
    gta_events: u64,
    gta_ratio: f64,
    avg_confidence: f64,
}

fn generate_sources_csv(
    workdir: &Path,
    domains: &HashMap<String, DomainRecord>,
) -> Result<()> {
    let mut writer = AtomicWriter::new(workdir.join("toronto_event_sources.csv"))?;
    writeln!(
        writer,
        "rank,domain,classification,total_events,gta_events,gta_ratio,avg_confidence"
    )?;

    // Include Confirmed and Likely domains
    let include_classes: HashSet<&str> =
        ["Confirmed", "Likely"].iter().copied().collect();

    let mut included: Vec<(&String, &DomainRecord)> = domains
        .iter()
        .filter(|(_, r)| include_classes.contains(r.classification.as_str()))
        .collect();

    included.sort_by(|a, b| {
        b.1.gta_events
            .cmp(&a.1.gta_events)
            .then(b.1.avg_confidence.partial_cmp(&a.1.avg_confidence).unwrap_or(std::cmp::Ordering::Equal))
    });

    for (rank, (domain, record)) in included.iter().enumerate() {
        writeln!(
            writer,
            "{},{},{},{},{},{:.3},{:.3}",
            rank + 1,
            domain,
            record.classification,
            record.total_events,
            record.gta_events,
            record.gta_ratio,
            record.avg_confidence,
        )?;
    }

    writer.commit()?;
    info!("Generated toronto_event_sources.csv ({} domains)", included.len());
    Ok(())
}

fn generate_samples_ndjson(
    workdir: &Path,
    domains: &HashMap<String, DomainRecord>,
) -> Result<()> {
    let include_domains: HashSet<&str> = domains
        .iter()
        .filter(|(_, r)| r.classification == "Confirmed" || r.classification == "Likely")
        .map(|(d, _)| d.as_str())
        .collect();

    let mut writer = AtomicWriter::new(workdir.join("toronto_event_samples.ndjson"))?;
    let mut domain_counts: HashMap<String, u32> = HashMap::new();

    let geofiltered_path = workdir.join("geofiltered_events.ndjson");
    if !geofiltered_path.exists() {
        writer.commit()?;
        return Ok(());
    }

    let file = std::fs::File::open(&geofiltered_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
            let domain = val
                .get("domain")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            if include_domains.contains(domain) {
                let count = domain_counts.entry(domain.to_string()).or_default();
                if *count < 3 {
                    writeln!(writer, "{}", line)?;
                    *count += 1;
                }
            }
        }
    }

    writer.commit()?;
    info!("Generated toronto_event_samples.ndjson");
    Ok(())
}

fn generate_review_csv(
    workdir: &Path,
    domains: &HashMap<String, DomainRecord>,
) -> Result<()> {
    let mut writer = AtomicWriter::new(workdir.join("manual_review_queue.csv"))?;
    writeln!(
        writer,
        "domain,classification,total_events,gta_events,gta_ratio,avg_confidence"
    )?;

    let review_classes: HashSet<&str> =
        ["Possible", "Review"].iter().copied().collect();

    let mut review: Vec<(&String, &DomainRecord)> = domains
        .iter()
        .filter(|(_, r)| review_classes.contains(r.classification.as_str()))
        .collect();

    review.sort_by(|a, b| b.1.gta_events.cmp(&a.1.gta_events));

    for (domain, record) in &review {
        writeln!(
            writer,
            "{},{},{},{},{:.3},{:.3}",
            domain,
            record.classification,
            record.total_events,
            record.gta_events,
            record.gta_ratio,
            record.avg_confidence,
        )?;
    }

    writer.commit()?;
    info!(
        "Generated manual_review_queue.csv ({} domains)",
        review.len()
    );
    Ok(())
}
