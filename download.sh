#!/usr/bin/env bash
set -euo pipefail

BASE_URL="https://data.dws.informatik.uni-mannheim.de/structureddata/2024-12/quads/classspecific/Event"
DATA_DIR="${1:-./data}"
PARTS_DIR="$DATA_DIR/parts"
JOBS="${2:-4}"

echo "=== commoncrawletl download ==="
echo "Data directory: $DATA_DIR"
echo "Download concurrency: $JOBS"
echo ""

mkdir -p "$PARTS_DIR"

# Download lookup and stats CSVs
echo "Downloading Event_lookup.csv..."
curl -fSL --progress-bar -o "$DATA_DIR/Event_lookup.csv" "$BASE_URL/Event_lookup.csv"

echo "Downloading Event_domain_stats.csv..."
curl -fSL --progress-bar -o "$DATA_DIR/Event_domain_stats.csv" "$BASE_URL/Event_domain_stats.csv"

echo "Downloading Event_sample.txt..."
curl -fSL --progress-bar -o "$DATA_DIR/Event_sample.txt" "$BASE_URL/Event_sample.txt"

# Download part files (133 files, ~20GB total)
# Use aria2c if available for parallel downloads, otherwise fall back to curl
if command -v aria2c &>/dev/null; then
    echo ""
    echo "Downloading 133 part files with aria2c ($JOBS concurrent)..."
    echo "Total size: ~20.8 GB"
    echo ""

    # Generate URL list
    URL_LIST=$(mktemp)
    for i in $(seq 0 132); do
        echo "$BASE_URL/part_${i}.gz"
        echo "  dir=$PARTS_DIR"
        echo "  out=part_${i}.gz"
    done > "$URL_LIST"

    aria2c \
        --input-file="$URL_LIST" \
        --max-concurrent-downloads="$JOBS" \
        --max-connection-per-server=1 \
        --continue=true \
        --auto-file-renaming=false \
        --console-log-level=notice \
        --summary-interval=30

    rm -f "$URL_LIST"
else
    echo ""
    echo "Downloading 133 part files with curl (sequential)..."
    echo "Total size: ~20.8 GB"
    echo "Tip: install aria2c for parallel downloads (run setup.sh first)"
    echo ""

    for i in $(seq 0 132); do
        DEST="$PARTS_DIR/part_${i}.gz"
        if [ -f "$DEST" ]; then
            echo "  part_${i}.gz already exists, skipping"
            continue
        fi
        echo "  Downloading part_${i}.gz ($(( i + 1 ))/133)..."
        curl -fSL --progress-bar -o "$DEST" "$BASE_URL/part_${i}.gz"
    done
fi

echo ""
echo "=== Download complete ==="
echo "Files in $DATA_DIR:"
ls -lh "$DATA_DIR"/Event_*.csv "$DATA_DIR"/Event_sample.txt 2>/dev/null || true
echo ""
echo "Parts: $(ls "$PARTS_DIR"/*.gz 2>/dev/null | wc -l | tr -d ' ') files"
du -sh "$PARTS_DIR" 2>/dev/null || true
echo ""
echo "Next step: see USAGE.md for pipeline execution commands"
