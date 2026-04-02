use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use tracing::info;

use crate::atomic::AtomicWriter;
use crate::checkpoint::Checkpoint;
use crate::error::{Error, Result};

/// Known Toronto/GTA institutional domains.
const KNOWN_INSTITUTIONS: &[&str] = &[
    "toronto.ca",
    "torontopubliclibrary.ca",
    "tpl.ca",
    "blogto.com",
    "nowtoronto.com",
    "thestar.com",
    "cp24.com",
    "citynews.ca",
    "toronto.com",
    "torontolife.com",
    "torontoist.com",
    "tocultrun.com",
    "ttc.ca",
    "ago.ca",
    "rom.on.ca",
    "harbourfrontcentre.com",
    "masseyhall.com",
    "roythomsonhall.com",
    "tiff.net",
    "cne.ca",
    "ontarioplace.com",
    "ripleyaquariums.com",
    "cntower.ca",
    "evergreen.ca",
    "artgalleryofontario.com",
    "torontobiennial.org",
    "luminatofestival.com",
    "nuitblanche.com",
    "torontojazz.com",
    "pride.to",
    "caribanatoronto.com",
    "tasteofthedanforth.com",
    "theex.com",
    "sfrequency.com",
    "mississauga.ca",
    "brampton.ca",
    "markham.ca",
    "vaughan.ca",
    "richmondhill.ca",
    "oakville.ca",
    "burlington.ca",
    "hamilton.ca",
    "pickering.ca",
    "ajax.ca",
    "whitby.ca",
    "oshawa.ca",
    "durham.ca",
    "peelregion.ca",
    "york.ca",
    "halton.ca",
];

/// Toronto/GTA keywords that signal relevance.
const TORONTO_KEYWORDS: &[&str] = &[
    "toronto",
    "torontonian",
    "tdot",
    "the6ix",
    "sixburgh",
    "yyz",
    "416",
];

/// GTA city/region keywords.
const GTA_KEYWORDS: &[&str] = &[
    "mississauga",
    "brampton",
    "markham",
    "vaughan",
    "richmondhill",
    "scarborough",
    "etobicoke",
    "northyork",
    "eastyork",
    "oakville",
    "burlington",
    "hamilton",
    "pickering",
    "ajax",
    "whitby",
    "oshawa",
    "newmarket",
    "aurora",
    "kingcity",
    "caledon",
    "milton",
    "haltonhills",
    "georgetownon",
];

/// Run Phase 1: Analyze domains from WDC lookup and stats CSVs.
pub fn run(
    workdir: &Path,
    lookup_path: &Path,
    stats_path: &Path,
    checkpoint: &mut Checkpoint,
    checkpoint_path: &Path,
) -> Result<()> {
    if checkpoint.phase1_complete {
        info!("Phase 1 already complete, skipping");
        return Ok(());
    }

    let known_set: HashSet<&str> = KNOWN_INSTITUTIONS.iter().copied().collect();

    // Load domains from lookup CSV
    let mut domains: HashMap<String, DomainInfo> = HashMap::new();
    load_lookup_csv(lookup_path, &mut domains)?;

    info!("Loaded {} domains from lookup CSV", domains.len());

    // Enrich with stats CSV
    load_stats_csv(stats_path, &mut domains)?;

    // Classify each domain
    let mut writer = AtomicWriter::new(workdir.join("domain_signals.csv"))?;
    writeln!(
        writer,
        "domain,classification,score,signals"
    )?;

    let mut positive = 0u32;
    let mut negative = 0u32;
    let mut neutral = 0u32;

    for (domain, info) in &domains {
        let (classification, score, signals) = classify_domain(domain, info, &known_set);

        writeln!(
            writer,
            "{},{},{},\"{}\"",
            domain,
            classification,
            score,
            signals.join("; ")
        )?;

        match classification.as_str() {
            "POSITIVE" => positive += 1,
            "NEGATIVE" => negative += 1,
            _ => neutral += 1,
        }
    }

    writer.commit()?;

    info!(
        "Phase 1 complete: {} positive, {} negative, {} neutral domains",
        positive, negative, neutral
    );

    checkpoint.phase1_complete = true;
    checkpoint.save(checkpoint_path)?;

    Ok(())
}

#[derive(Debug, Default)]
struct DomainInfo {
    event_count: u64,
    part_files: Vec<u32>,
}

fn load_lookup_csv(path: &Path, domains: &mut HashMap<String, DomainInfo>) -> Result<()> {
    if !path.exists() {
        return Err(Error::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)?;

    for result in rdr.records() {
        let record = result?;
        // Expected: domain, part_number, ...
        if let Some(domain) = record.get(0) {
            let domain = domain.trim().to_lowercase();
            if domain.is_empty() {
                continue;
            }
            let part_num = record
                .get(1)
                .and_then(|s| s.trim().parse::<u32>().ok());

            let entry = domains.entry(domain).or_default();
            if let Some(pn) = part_num {
                entry.part_files.push(pn);
            }
        }
    }

    Ok(())
}

fn load_stats_csv(path: &Path, domains: &mut HashMap<String, DomainInfo>) -> Result<()> {
    if !path.exists() {
        return Err(Error::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)?;

    for result in rdr.records() {
        let record = result?;
        if let (Some(domain), Some(count)) = (record.get(0), record.get(1)) {
            let domain = domain.trim().to_lowercase();
            if let Ok(count) = count.trim().parse::<u64>() {
                domains.entry(domain).or_default().event_count = count;
            }
        }
    }

    Ok(())
}

fn classify_domain(
    domain: &str,
    _info: &DomainInfo,
    known_set: &HashSet<&str>,
) -> (String, i32, Vec<String>) {
    let mut score: i32 = 0;
    let mut signals = Vec::new();
    let domain_lower = domain.to_lowercase();

    // Check known institutions (exact match or subdomain)
    let is_known = known_set.iter().any(|inst| {
        domain_lower == *inst || domain_lower.ends_with(&format!(".{inst}"))
    });

    if is_known {
        score += 100;
        signals.push("known_institution".into());
    }

    // Check Toronto keywords in domain name
    let domain_normalized = domain_lower.replace(['.', '-', '_'], "");
    for kw in TORONTO_KEYWORDS {
        if domain_normalized.contains(kw) {
            score += 50;
            signals.push(format!("toronto_keyword:{kw}"));
            break;
        }
    }

    // Check GTA keywords
    for kw in GTA_KEYWORDS {
        if domain_normalized.contains(kw) {
            score += 30;
            signals.push(format!("gta_keyword:{kw}"));
            break;
        }
    }

    // TLD signals
    if domain_lower.ends_with(".ca") {
        score += 10;
        signals.push("tld_ca".into());
    } else if domain_lower.ends_with(".to") {
        score += 20;
        signals.push("tld_to".into());
    }

    // Negative signals: non-GTA Canadian cities
    let non_gta_cities = [
        "vancouver",
        "calgary",
        "edmonton",
        "winnipeg",
        "ottawa",
        "montreal",
        "quebec",
        "halifax",
        "victoria",
        "saskatoon",
        "regina",
    ];
    for city in &non_gta_cities {
        if domain_normalized.contains(city) {
            score -= 25;
            signals.push(format!("non_gta_city:{city}"));
            break;
        }
    }

    // Classification
    let classification = if score >= 50 {
        "POSITIVE"
    } else if score <= -10 {
        "NEGATIVE"
    } else {
        "NEUTRAL"
    };

    (classification.into(), score, signals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_known_institution() {
        let known: HashSet<&str> = KNOWN_INSTITUTIONS.iter().copied().collect();
        let info = DomainInfo::default();

        let (cls, score, _) = classify_domain("toronto.ca", &info, &known);
        assert_eq!(cls, "POSITIVE");
        assert!(score >= 100);
    }

    #[test]
    fn test_classify_toronto_keyword() {
        let known: HashSet<&str> = KNOWN_INSTITUTIONS.iter().copied().collect();
        let info = DomainInfo::default();

        let (cls, score, _) = classify_domain("torontoevents.org", &info, &known);
        assert_eq!(cls, "POSITIVE");
        assert!(score >= 50);
    }

    #[test]
    fn test_classify_negative() {
        let known: HashSet<&str> = KNOWN_INSTITUTIONS.iter().copied().collect();
        let info = DomainInfo::default();

        let (cls, score, _) = classify_domain("vancouverevents.com", &info, &known);
        assert_eq!(cls, "NEGATIVE");
        assert!(score < 0);
    }

    #[test]
    fn test_classify_neutral() {
        let known: HashSet<&str> = KNOWN_INSTITUTIONS.iter().copied().collect();
        let info = DomainInfo::default();

        let (cls, _score, _) = classify_domain("randomsite.com", &info, &known);
        assert_eq!(cls, "NEUTRAL");
    }
}
