use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use rayon::prelude::*;
use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};
use crate::extract::entity::extract_event;
use crate::nquads::parser::{parse_line, SubjectGrouper};
use crate::progress;

/// Run Phase 3: Extract events from gzipped N-Quads part files.
pub fn run(
    workdir: &Path,
    parts_dir: &Path,
    jobs: usize,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase3_complete {
        info!("Phase 3 already complete, skipping");
        return Ok(());
    }

    if !checkpoint.phase1_complete {
        return Err(Error::PhaseNotComplete { phase: 1 });
    }

    // Load domain signals for filtering
    let domain_signals = load_domain_signals(workdir)?;
    let negative_domains: HashSet<String> = domain_signals
        .iter()
        .filter(|(_, cls)| cls.as_str() == "NEGATIVE")
        .map(|(d, _)| d.clone())
        .collect();

    info!(
        "Loaded {} domain signals ({} negative)",
        domain_signals.len(),
        negative_domains.len()
    );

    // Find part files
    let mut part_files = find_part_files(parts_dir)?;
    part_files.sort();

    if part_files.is_empty() {
        return Err(Error::MissingInput {
            description: format!("no part-*.gz files found in {}", parts_dir.display()),
        });
    }

    info!("Found {} part files", part_files.len());

    let output_dir = workdir.join("extracted");
    std::fs::create_dir_all(&output_dir)?;

    // Configure rayon thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Other {
            message: format!("failed to create thread pool: {e}"),
        })?;

    // Process parts in parallel
    let results: Vec<Result<(u32, u64)>> = pool.install(|| {
        part_files
            .par_iter()
            .map(|(part_num, path)| {
                // Check if already done (read-only check, safe across threads)
                if checkpoint.is_part_complete(*part_num) {
                    info!(part = part_num, "Part already complete, skipping");
                    return Ok((*part_num, 0));
                }

                let count = process_part(
                    *part_num,
                    path,
                    &output_dir,
                    &negative_domains,
                )?;

                Ok((*part_num, count))
            })
            .collect()
    });

    // Update checkpoint sequentially
    let mut total_events = 0u64;
    for result in results {
        let (part_num, count) = result?;
        total_events += count;
        if !checkpoint.is_part_complete(part_num) && count > 0 {
            checkpoint.mark_part_complete(part_num, checkpoint_path)?;
        }
    }

    checkpoint.phase3_complete = true;
    checkpoint.save(checkpoint_path)?;

    info!("Phase 3 complete: extracted {total_events} events");
    Ok(())
}

fn process_part(
    part_num: u32,
    path: &Path,
    output_dir: &Path,
    negative_domains: &HashSet<String>,
) -> Result<u64> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::with_capacity(64 * 1024, decoder);

    let output_path = output_dir.join(format!("events_part_{part_num}.ndjson"));
    let mut writer = AtomicWriter::new(&output_path)?;

    let mut grouper = SubjectGrouper::new(50_000);
    let mut blank_nodes: HashMap<String, Vec<crate::nquads::types::Quad>> = HashMap::new();
    let mut event_count = 0u64;
    let mut line_count = 0u64;

    let pb = progress::spinner(&format!("Part {part_num}"));

    for line in reader.lines() {
        let line = line?;
        line_count += 1;

        if line_count % 100_000 == 0 {
            pb.set_message(format!("{line_count} lines, {event_count} events"));
        }

        let quad = match parse_line(&line) {
            Ok(q) => q,
            Err(_) => continue,
        };

        // Domain filtering: skip negative domains early
        if let Some(domain) = quad.graph_domain() {
            if negative_domains.contains(&domain) {
                continue;
            }
        }

        // Track blank node definitions
        if quad.subject.is_blank_node() {
            let key = quad.subject.as_str_value().to_string();
            let entry = blank_nodes.entry(key).or_default();
            entry.push(quad.clone());

            // Cap blank node buffer
            if blank_nodes.len() > 500_000 {
                // Evict oldest half
                let keys: Vec<String> = blank_nodes.keys().take(250_000).cloned().collect();
                for k in keys {
                    blank_nodes.remove(&k);
                }
            }
        }

        // Group by subject and extract events from completed groups
        let completed_groups = grouper.push(quad);
        for group in completed_groups {
            if let Some(event) = extract_event(&group, &blank_nodes, part_num) {
                let json = serde_json::to_string(&event)?;
                writeln!(writer, "{json}")?;
                event_count += 1;
            }
        }
    }

    // Flush remaining groups
    for group in grouper.flush() {
        if let Some(event) = extract_event(&group, &blank_nodes, part_num) {
            let json = serde_json::to_string(&event)?;
            writeln!(writer, "{json}")?;
            event_count += 1;
        }
    }

    writer.commit()?;
    pb.finish_with_message(format!("{event_count} events from {line_count} lines"));

    info!(part = part_num, events = event_count, lines = line_count, "Part complete");
    Ok(event_count)
}

fn load_domain_signals(workdir: &Path) -> Result<HashMap<String, String>> {
    let path = workdir.join("domain_signals.csv");
    if !path.exists() {
        return Err(Error::FileNotFound { path });
    }

    let mut signals = HashMap::new();
    let mut rdr = csv::Reader::from_path(&path)?;

    for result in rdr.records() {
        let record = result?;
        if let (Some(domain), Some(classification)) = (record.get(0), record.get(1)) {
            signals.insert(domain.to_string(), classification.to_string());
        }
    }

    Ok(signals)
}

fn find_part_files(dir: &Path) -> Result<Vec<(u32, PathBuf)>> {
    if !dir.exists() {
        return Err(Error::FileNotFound {
            path: dir.to_path_buf(),
        });
    }

    let mut parts = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Match patterns like "part-00000.gz" or "part_0.gz"
            if name.ends_with(".gz") && name.starts_with("part") {
                if let Some(num) = extract_part_number(name) {
                    parts.push((num, path));
                }
            }
        }
    }

    Ok(parts)
}

fn extract_part_number(name: &str) -> Option<u32> {
    // Try "part-NNNNN.gz" or "part_N.gz" or "part-N.nq.gz"
    let stem = name.strip_suffix(".gz")?;
    let stem = stem.strip_suffix(".nq").unwrap_or(stem);
    let num_part = stem
        .strip_prefix("part-")
        .or_else(|| stem.strip_prefix("part_"))?;
    num_part.parse().ok()
}
