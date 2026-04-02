use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};
use crate::extract::types::ExtractedEvent;
use crate::geo::constants::*;
use crate::geo::types::*;

/// Run Phase 4: Geo-filter extracted events for Toronto/GTA.
pub fn run(
    workdir: &Path,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase4_complete {
        info!("Phase 4 already complete, skipping");
        return Ok(());
    }

    if !checkpoint.phase3_complete {
        return Err(Error::PhaseNotComplete { phase: 3 });
    }

    let extracted_dir = workdir.join("extracted");
    if !extracted_dir.exists() {
        return Err(Error::FileNotFound {
            path: extracted_dir,
        });
    }

    let mut writer = AtomicWriter::new(workdir.join("geofiltered_events.ndjson"))?;
    let mut total_events = 0u64;
    let mut matched_events = 0u64;

    // Process all extracted NDJSON files
    let mut entries: Vec<_> = std::fs::read_dir(&extracted_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "ndjson")
        })
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let file = std::fs::File::open(&path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            total_events += 1;

            let event: ExtractedEvent = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let geo_result = match_event(&event);
            if geo_result.matched {
                matched_events += 1;

                // Write event with geo match info
                let mut value: serde_json::Value = serde_json::from_str(&line)?;
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "geo_confidence".into(),
                        serde_json::Value::from(geo_result.confidence),
                    );
                    obj.insert(
                        "geo_strategy".into(),
                        serde_json::Value::from(geo_result.strategy.to_string()),
                    );
                    obj.insert(
                        "geo_details".into(),
                        serde_json::Value::from(geo_result.details),
                    );
                }
                let json = serde_json::to_string(&value)?;
                writeln!(writer, "{json}")?;
            }
        }
    }

    writer.commit()?;

    info!(
        "Phase 4 complete: {matched_events}/{total_events} events matched Toronto/GTA"
    );

    checkpoint.phase4_complete = true;
    checkpoint.save(checkpoint_path)?;

    Ok(())
}

/// Match an event against Toronto/GTA geo criteria.
/// Tries strategies in order of confidence: postal code, bounding box, locality, region.
pub fn match_event(event: &ExtractedEvent) -> GeoMatchResult {
    // Strategy 1: Postal code
    if let Some(result) = match_postal_code(event) {
        return result;
    }

    // Strategy 2: Bounding box (lat/lon)
    if let Some(result) = match_bounding_box(event) {
        return result;
    }

    // Strategy 3: Locality names
    if let Some(result) = match_locality(event) {
        return result;
    }

    // Strategy 4: Region (weak signal)
    if let Some(result) = match_region(event) {
        return result;
    }

    GeoMatchResult::no_match()
}

fn match_postal_code(event: &ExtractedEvent) -> Option<GeoMatchResult> {
    let postal = event
        .location
        .as_ref()?
        .address
        .as_ref()?
        .postal_code
        .as_ref()?;

    let postal_upper = postal.trim().to_uppercase().replace(' ', "");
    if postal_upper.len() < 3 {
        return None;
    }

    let fsa = &postal_upper[..3];

    // Check Toronto M* prefixes
    for prefix in TORONTO_POSTAL_PREFIXES {
        if fsa.starts_with(prefix) {
            return Some(GeoMatchResult {
                matched: true,
                confidence: 0.95,
                strategy: MatchStrategy::PostalCode,
                details: format!("Toronto postal code: {fsa}"),
            });
        }
    }

    // Check GTA L* prefixes
    for prefix in GTA_POSTAL_PREFIXES {
        if fsa.starts_with(prefix) {
            return Some(GeoMatchResult {
                matched: true,
                confidence: 0.90,
                strategy: MatchStrategy::PostalCode,
                details: format!("GTA postal code: {fsa}"),
            });
        }
    }

    None
}

fn match_bounding_box(event: &ExtractedEvent) -> Option<GeoMatchResult> {
    let loc = event.location.as_ref()?;
    let lat = loc.latitude?;
    let lon = loc.longitude?;

    if TORONTO_BBOX.contains(lat, lon) {
        return Some(GeoMatchResult {
            matched: true,
            confidence: 0.95,
            strategy: MatchStrategy::BoundingBox,
            details: format!("Toronto bbox: ({lat}, {lon})"),
        });
    }

    if GTA_BBOX.contains(lat, lon) {
        return Some(GeoMatchResult {
            matched: true,
            confidence: 0.85,
            strategy: MatchStrategy::BoundingBox,
            details: format!("GTA bbox: ({lat}, {lon})"),
        });
    }

    None
}

fn match_locality(event: &ExtractedEvent) -> Option<GeoMatchResult> {
    // Check address locality
    if let Some(locality) = event
        .location
        .as_ref()
        .and_then(|l| l.address.as_ref())
        .and_then(|a| a.locality.as_ref())
    {
        if let Some(result) = check_locality_name(locality) {
            return Some(result);
        }
    }

    // Check location name
    if let Some(name) = event.location.as_ref().and_then(|l| l.name.as_ref()) {
        if let Some(result) = check_locality_name(name) {
            return Some(result);
        }
    }

    None
}

fn check_locality_name(text: &str) -> Option<GeoMatchResult> {
    let text_lower = text.to_lowercase();

    for &(name, confidence) in LOCALITY_NAMES {
        if text_lower.contains(name) {
            return Some(GeoMatchResult {
                matched: true,
                confidence,
                strategy: MatchStrategy::Locality,
                details: format!("Locality match: \"{name}\" in \"{text}\""),
            });
        }
    }

    None
}

fn match_region(event: &ExtractedEvent) -> Option<GeoMatchResult> {
    let region = event
        .location
        .as_ref()?
        .address
        .as_ref()?
        .region
        .as_ref()?;

    let region_lower = region.trim().to_lowercase();
    if region_lower == "on" || region_lower == "ontario" {
        return Some(GeoMatchResult {
            matched: true,
            confidence: 0.30,
            strategy: MatchStrategy::Region,
            details: format!("Region: {region}"),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::types::{AddressInfo, LocationInfo};

    fn make_event_with_postal(postal: &str) -> ExtractedEvent {
        ExtractedEvent {
            name: Some("Test Event".into()),
            description: None,
            start_date: None,
            end_date: None,
            url: None,
            event_type: None,
            location: Some(LocationInfo {
                name: None,
                address: Some(AddressInfo {
                    street: None,
                    locality: None,
                    region: None,
                    postal_code: Some(postal.into()),
                    country: None,
                }),
                latitude: None,
                longitude: None,
            }),
            organizer: None,
            domain: "example.com".into(),
            source_url: "http://example.com".into(),
            part_number: 0,
        }
    }

    #[test]
    fn test_toronto_postal() {
        let event = make_event_with_postal("M5V 3L9");
        let result = match_event(&event);
        assert!(result.matched);
        assert_eq!(result.strategy, MatchStrategy::PostalCode);
        assert!(result.confidence >= 0.95);
    }

    #[test]
    fn test_gta_postal() {
        let event = make_event_with_postal("L5B 3C2");
        let result = match_event(&event);
        assert!(result.matched);
        assert_eq!(result.strategy, MatchStrategy::PostalCode);
    }

    #[test]
    fn test_non_gta_postal() {
        let event = make_event_with_postal("V6B 1A1"); // Vancouver
        let result = match_event(&event);
        assert!(!result.matched);
    }

    #[test]
    fn test_toronto_bbox() {
        let mut event = make_event_with_postal(""); // No postal
        event.location = Some(LocationInfo {
            name: None,
            address: None,
            latitude: Some(43.65),
            longitude: Some(-79.38),
        });
        let result = match_event(&event);
        assert!(result.matched);
        assert_eq!(result.strategy, MatchStrategy::BoundingBox);
    }

    #[test]
    fn test_locality_match() {
        let mut event = make_event_with_postal("");
        event.location = Some(LocationInfo {
            name: None,
            address: Some(AddressInfo {
                street: None,
                locality: Some("Toronto".into()),
                region: None,
                postal_code: None,
                country: None,
            }),
            latitude: None,
            longitude: None,
        });
        let result = match_event(&event);
        assert!(result.matched);
        assert_eq!(result.strategy, MatchStrategy::Locality);
    }
}
