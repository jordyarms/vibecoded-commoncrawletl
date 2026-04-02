# commoncrawletl — CLI Reference

## Quick Start

```bash
# 1. Setup (install Rust, build binary)
./setup.sh

# 2. Download WDC Event data (~20GB)
./download.sh ./data 4

# 3. Run full pipeline
RUST_LOG=info ./target/release/commoncrawletl -w ./data run \
  --lookup ./data/Event_lookup.csv \
  --stats ./data/Event_domain_stats.csv \
  --parts-dir ./data/parts \
  -j $(nproc)
```

## Phase-by-Phase Execution

Run phases individually if you want to inspect intermediate outputs or re-run a specific step.

### Phase 1 — Domain Analysis

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data analyze \
  --lookup ./data/Event_lookup.csv \
  --stats ./data/Event_domain_stats.csv
```

Output: `data/domain_signals.csv`

### Phase 2 — Part Prioritization

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data prioritize
```

Output: `data/part_priority.csv`

### Phase 3 — Event Extraction

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data extract \
  --parts-dir ./data/parts \
  -j 8
```

Output: `data/extracted/events_part_N.ndjson` (one per part file)

### Phase 4 — Geo-Filtering

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data geofilter
```

Output: `data/geofiltered_events.ndjson`

### Phase 5 — Domain Scoring

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data score
```

Output: `data/domain_scores.csv`

### Phase 6 — Output Generation

```bash
RUST_LOG=info ./target/release/commoncrawletl -w ./data output
```

Outputs:
- `data/toronto_event_sources.csv` — ranked domain list
- `data/toronto_event_samples.ndjson` — top 3 events per confirmed domain
- `data/manual_review_queue.csv` — domains needing human review

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --workdir` | `.` | Working directory for all input/output files |
| `--checkpoint` | `checkpoint.json` | Checkpoint file path (relative to workdir) |
| `-j, --jobs` | `4` | Parallel workers for extract phase |

## Logging

Control log verbosity with `RUST_LOG`:

```bash
RUST_LOG=debug ./target/release/commoncrawletl ...   # verbose
RUST_LOG=info ./target/release/commoncrawletl ...    # normal
RUST_LOG=warn ./target/release/commoncrawletl ...    # quiet
```

## Resuming After a Crash

The pipeline checkpoints after each completed phase and each completed part file.
Just re-run the same command — it will skip already-completed work.

```bash
# Check current progress
cat ./data/checkpoint.json | python3 -m json.tool

# Resume from where it left off
RUST_LOG=info ./target/release/commoncrawletl -w ./data run \
  --lookup ./data/Event_lookup.csv \
  --stats ./data/Event_domain_stats.csv \
  --parts-dir ./data/parts \
  -j $(nproc)
```

## Resource Guidelines

| Workers (`-j`) | RAM Usage | Recommended For |
|-----------------|-----------|-----------------|
| 2 | ~500 MB | 1 GB VM |
| 4 | ~1 GB | 2-4 GB VM |
| 8 | ~1.5 GB | 4-8 GB VM |
| 16 | ~3 GB | 8+ GB VM |

CPU-bound workload — more cores = faster. Disk I/O matters for reading gzipped parts.
