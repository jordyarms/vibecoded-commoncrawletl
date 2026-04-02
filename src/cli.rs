use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "commoncrawletl", version, about = "Toronto Events ETL Pipeline")]
pub struct Cli {
    /// Working directory for input/output files
    #[arg(short, long, default_value = ".")]
    pub workdir: PathBuf,

    /// Checkpoint file path (relative to workdir)
    #[arg(long, default_value = "checkpoint.json")]
    pub checkpoint: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Phase 1: Analyze domains from WDC lookup/stats CSVs
    Analyze {
        /// Path to Event_lookup.csv
        #[arg(long)]
        lookup: PathBuf,

        /// Path to Event_domain_stats.csv
        #[arg(long)]
        stats: PathBuf,
    },

    /// Phase 2: Prioritize part files by domain signals
    Prioritize,

    /// Phase 3: Extract events from gzipped N-Quads part files
    Extract {
        /// Directory containing part-*.gz files
        #[arg(long)]
        parts_dir: PathBuf,

        /// Number of parallel workers
        #[arg(short, long, default_value = "4")]
        jobs: usize,
    },

    /// Phase 4: Geo-filter extracted events for Toronto/GTA
    Geofilter,

    /// Phase 5: Score domains based on geo-filtered results
    Score,

    /// Phase 6: Generate final output files
    Output,

    /// Run the full pipeline end-to-end
    Run {
        /// Path to Event_lookup.csv
        #[arg(long)]
        lookup: PathBuf,

        /// Path to Event_domain_stats.csv
        #[arg(long)]
        stats: PathBuf,

        /// Directory containing part-*.gz files
        #[arg(long)]
        parts_dir: PathBuf,

        /// Number of parallel workers
        #[arg(short, long, default_value = "4")]
        jobs: usize,
    },
}
