use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};

/// Run Phase 2: Prioritize part files by domain signals.
pub fn run(
    workdir: &Path,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase2_complete {
        info!("Phase 2 already complete, skipping");
        return Ok(());
    }

    if !checkpoint.phase1_complete {
        return Err(Error::PhaseNotComplete { phase: 1 });
    }

    // Load domain signals
    let signals_path = workdir.join("domain_signals.csv");
    if !signals_path.exists() {
        return Err(Error::FileNotFound {
            path: signals_path,
        });
    }

    // Parse domain signals and their part associations
    // We need the original lookup CSV to know which parts each domain appears in.
    // For now, we aggregate from the domain_signals.csv which has domain + classification + score.
    let mut rdr = csv::Reader::from_path(&signals_path)?;

    // We need part info — re-read from domain_signals if stored, or compute from lookup.
    // Since the lookup CSV maps domains to parts, we aggregate:
    // For each part, count positive domains, toronto keywords, total score.

    // Load the lookup CSV to get domain→part mappings
    let lookup_path = workdir.join("Event_lookup.csv");
    let domain_parts = if lookup_path.exists() {
        load_domain_parts(&lookup_path)?
    } else {
        // Fallback: no part data available
        HashMap::new()
    };

    // Load domain scores
    let mut domain_scores: HashMap<String, (String, i32)> = HashMap::new();
    for result in rdr.records() {
        let record = result?;
        if let (Some(domain), Some(classification), Some(score)) =
            (record.get(0), record.get(1), record.get(2))
        {
            if let Ok(score) = score.parse::<i32>() {
                domain_scores.insert(domain.to_string(), (classification.to_string(), score));
            }
        }
    }

    // Aggregate per part
    let mut part_stats: HashMap<u32, PartStats> = HashMap::new();

    for (domain, parts) in &domain_parts {
        let (classification, score) = domain_scores
            .get(domain)
            .cloned()
            .unwrap_or(("NEUTRAL".into(), 0));

        for &part in parts {
            let stats = part_stats.entry(part).or_default();
            stats.total_domains += 1;
            stats.total_score += score as i64;

            match classification.as_str() {
                "POSITIVE" => stats.positive_domains += 1,
                "NEGATIVE" => stats.negative_domains += 1,
                _ => {}
            }

            let domain_lower = domain.to_lowercase();
            if domain_lower.contains("toronto") {
                stats.toronto_keyword_count += 1;
            }
        }
    }

    // Sort by priority
    let mut parts: Vec<(u32, PartStats)> = part_stats.into_iter().collect();
    parts.sort_by(|a, b| {
        b.1.positive_domains
            .cmp(&a.1.positive_domains)
            .then(b.1.toronto_keyword_count.cmp(&a.1.toronto_keyword_count))
            .then(b.1.total_score.cmp(&a.1.total_score))
    });

    // Write output
    let mut writer = AtomicWriter::new(workdir.join("part_priority.csv"))?;
    writeln!(
        writer,
        "part_number,priority_rank,positive_domains,negative_domains,total_domains,toronto_keywords,total_score"
    )?;

    for (rank, (part_num, stats)) in parts.iter().enumerate() {
        writeln!(
            writer,
            "{},{},{},{},{},{},{}",
            part_num,
            rank + 1,
            stats.positive_domains,
            stats.negative_domains,
            stats.total_domains,
            stats.toronto_keyword_count,
            stats.total_score,
        )?;
    }

    writer.commit()?;

    info!("Phase 2 complete: prioritized {} parts", parts.len());

    checkpoint.phase2_complete = true;
    checkpoint.save(checkpoint_path)?;

    Ok(())
}

#[derive(Debug, Default)]
struct PartStats {
    positive_domains: u32,
    negative_domains: u32,
    total_domains: u32,
    toronto_keyword_count: u32,
    total_score: i64,
}

fn load_domain_parts(path: &Path) -> Result<HashMap<String, Vec<u32>>> {
    let mut map: HashMap<String, Vec<u32>> = HashMap::new();
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)?;

    for result in rdr.records() {
        let record = result?;
        if let (Some(domain), Some(part)) = (record.get(0), record.get(1)) {
            let domain = domain.trim().to_lowercase();
            if let Ok(part_num) = part.trim().parse::<u32>() {
                map.entry(domain).or_default().push(part_num);
            }
        }
    }

    Ok(map)
}
