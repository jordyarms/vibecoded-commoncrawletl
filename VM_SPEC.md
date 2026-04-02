# VM Resource Request — commoncrawletl Pipeline

## Purpose

Process the Web Data Commons (WDC) Event corpus (~20GB compressed, ~2B N-Quads) to identify Toronto/GTA event sources. Single-run batch job — VM can be torn down after pipeline completes.

## Recommended Spec

| Resource | Minimum | Recommended | Notes |
|----------|---------|-------------|-------|
| **vCPUs** | 4 | 8 | CPU-bound gzip decompression + parsing. Linear scaling up to 16 cores. |
| **RAM** | 4 GB | 8 GB | ~170 MB per worker thread + OS overhead. 8 GB comfortably runs 16 workers. |
| **Disk** | 50 GB | 75 GB | 21 GB input (compressed) + ~15-20 GB extracted NDJSON + headroom. SSD preferred. |
| **OS** | Ubuntu 22.04+ | Ubuntu 24.04 LTS | Any modern Linux. Needs `build-essential`, `curl`, `aria2`. |
| **Network** | 100 Mbps | 1 Gbps | One-time 21 GB download from `data.dws.informatik.uni-mannheim.de`. |

## Cloud Instance Equivalents

| Provider | Instance Type | vCPU | RAM | Cost (on-demand) |
|----------|--------------|------|-----|-------------------|
| **AWS** | c6i.2xlarge | 8 | 16 GB | ~$0.34/hr |
| **AWS (spot)** | c6i.2xlarge | 8 | 16 GB | ~$0.10/hr |
| **GCP** | c2-standard-8 | 8 | 32 GB | ~$0.33/hr |
| **Azure** | Standard_F8s_v2 | 8 | 16 GB | ~$0.34/hr |

Spot/preemptible is fine — the pipeline checkpoints after each part file and resumes automatically.

## Disk

- Attached SSD (gp3/pd-ssd) preferred over HDD — pipeline reads 20 GB sequentially but writes many small NDJSON files.
- No persistent storage needed after outputs are copied off. Ephemeral/local SSD works.

## Runtime Estimate

| Phase | Duration (8 vCPU) |
|-------|-------------------|
| Download (1 Gbps) | ~3-5 min |
| Phase 1-2 (analyze + prioritize) | < 1 min |
| Phase 3 (extract 133 parts) | ~30-90 min |
| Phase 4-6 (geofilter + score + output) | ~5-10 min |
| **Total** | **~1-2 hours** |

## Access Requirements

- Outbound HTTPS to `data.dws.informatik.uni-mannheim.de` (data download)
- Outbound HTTPS to `static.rust-lang.org` (Rust installer) and `crates.io` (build dependencies)
- SSH access for operator
- No inbound ports needed

## Setup

Everything is automated. After SSH:

```bash
git clone <repo-url> && cd commoncrawletl
./setup.sh        # installs Rust + deps, builds binary (~2 min)
./download.sh ./data 8   # downloads 21 GB of input data
RUST_LOG=info ./target/release/commoncrawletl -w ./data run \
  --lookup ./data/Event_lookup.csv \
  --stats ./data/Event_domain_stats.csv \
  --parts-dir ./data/parts \
  -j $(nproc)
```

## Outputs to Retrieve

After completion, copy these files off the VM (~few MB total):

- `data/toronto_event_sources.csv` — ranked Toronto event domain list
- `data/toronto_event_samples.ndjson` — sample events per domain
- `data/manual_review_queue.csv` — domains needing human review
- `data/domain_scores.csv` — full domain scoring details
