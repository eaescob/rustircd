#!/bin/bash
# Benchmark Comparison Script
# Compares performance between current branch and baseline (main)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
THRESHOLD=5
BASELINE_BRANCH="main"
CURRENT_BRANCH=$(git branch --show-current)
OUTPUT_DIR="target/bench-compare"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --baseline)
            BASELINE_BRANCH="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --threshold N    Fail if performance regresses by more than N% (default: 5)"
            echo "  --baseline BRANCH  Compare against BRANCH (default: main)"
            echo "  --output DIR     Output directory for results (default: target/bench-compare)"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}==============================================================${NC}"
echo -e "${GREEN}RustIRCd Benchmark Comparison${NC}"
echo -e "${GREEN}==============================================================${NC}"
echo ""
echo "Current branch: $CURRENT_BRANCH"
echo "Baseline branch: $BASELINE_BRANCH"
echo "Regression threshold: ${THRESHOLD}%"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if we have uncommitted changes
if [[ -n $(git status -s) ]]; then
    echo -e "${YELLOW}Warning: You have uncommitted changes${NC}"
    echo "These changes will be included in the benchmark"
    echo ""
fi

# Run benchmarks on current branch
echo -e "${GREEN}Step 1: Running benchmarks on current branch ($CURRENT_BRANCH)...${NC}"
cargo bench --bench benchmarks -- --save-baseline current > "$OUTPUT_DIR/current.log" 2>&1
echo "✓ Current branch benchmarks complete"
echo ""

# Stash any changes
if [[ -n $(git status -s) ]]; then
    echo "Stashing uncommitted changes..."
    git stash push -m "bench-compare temp stash"
    STASHED=1
else
    STASHED=0
fi

# Switch to baseline branch
echo -e "${GREEN}Step 2: Switching to baseline branch ($BASELINE_BRANCH)...${NC}"
git checkout "$BASELINE_BRANCH" > /dev/null 2>&1
echo "✓ Switched to $BASELINE_BRANCH"
echo ""

# Run benchmarks on baseline
echo -e "${GREEN}Step 3: Running benchmarks on baseline branch...${NC}"
cargo bench --bench benchmarks -- --save-baseline baseline > "$OUTPUT_DIR/baseline.log" 2>&1
echo "✓ Baseline benchmarks complete"
echo ""

# Switch back to original branch
echo "Switching back to $CURRENT_BRANCH..."
git checkout "$CURRENT_BRANCH" > /dev/null 2>&1

# Restore stashed changes
if [[ $STASHED -eq 1 ]]; then
    echo "Restoring uncommitted changes..."
    git stash pop > /dev/null 2>&1
fi
echo ""

# Compare results using criterion
echo -e "${GREEN}Step 4: Comparing results...${NC}"
cargo bench --bench benchmarks -- --baseline baseline > "$OUTPUT_DIR/comparison.log" 2>&1

# Parse comparison results
echo ""
echo -e "${GREEN}==============================================================${NC}"
echo -e "${GREEN}COMPARISON RESULTS${NC}"
echo -e "${GREEN}==============================================================${NC}"
echo ""

# Extract performance changes from comparison log
# Look for lines like: "change: [-5.1234% -3.1234% -1.1234%]" or "change: [+1.1234% +3.1234% +5.1234%]"
REGRESSIONS=0
IMPROVEMENTS=0
MAX_REGRESSION=0

while IFS= read -r line; do
    if [[ $line =~ change:.*\[.*\+([0-9]+\.[0-9]+)% ]]; then
        # Performance regression (slower)
        CHANGE=${BASH_REMATCH[1]}
        CHANGE_INT=${CHANGE%.*}
        
        if (( $(echo "$CHANGE > $MAX_REGRESSION" | bc -l) )); then
            MAX_REGRESSION=$CHANGE
        fi
        
        if (( $(echo "$CHANGE > $THRESHOLD" | bc -l) )); then
            REGRESSIONS=$((REGRESSIONS + 1))
            BENCHMARK_NAME=$(echo "$line" | grep -oP '^[^/]+/[^/]+' || echo "unknown")
            echo -e "${RED}REGRESSION: $BENCHMARK_NAME +${CHANGE}%${NC}"
        fi
    elif [[ $line =~ change:.*\[-([0-9]+\.[0-9]+)% ]]; then
        # Performance improvement (faster)
        CHANGE=${BASH_REMATCH[1]}
        IMPROVEMENTS=$((IMPROVEMENTS + 1))
        BENCHMARK_NAME=$(echo "$line" | grep -oP '^[^/]+/[^/]+' || echo "unknown")
        echo -e "${GREEN}IMPROVEMENT: $BENCHMARK_NAME -${CHANGE}%${NC}"
    fi
done < "$OUTPUT_DIR/comparison.log"

echo ""
echo -e "${GREEN}==============================================================${NC}"
echo -e "${GREEN}SUMMARY${NC}"
echo -e "${GREEN}==============================================================${NC}"
echo ""
echo "Improvements: $IMPROVEMENTS"
echo "Regressions: $REGRESSIONS"
echo "Threshold: ${THRESHOLD}%"
echo "Max regression: ${MAX_REGRESSION}%"
echo ""

# Generate summary report
cat > "$OUTPUT_DIR/summary.txt" << EOF
RustIRCd Benchmark Comparison Summary
=====================================

Date: $(date)
Current branch: $CURRENT_BRANCH
Baseline branch: $BASELINE_BRANCH
Regression threshold: ${THRESHOLD}%

Results:
  Improvements: $IMPROVEMENTS
  Regressions: $REGRESSIONS
  Max regression: ${MAX_REGRESSION}%

Status: $(if [[ $REGRESSIONS -gt 0 ]]; then echo "FAILED"; else echo "PASSED"; fi)

Full logs available in: $OUTPUT_DIR/
  - current.log: Current branch benchmark results
  - baseline.log: Baseline branch benchmark results
  - comparison.log: Detailed comparison

To view detailed criterion reports:
  open target/criterion/report/index.html
EOF

echo "Summary report saved to: $OUTPUT_DIR/summary.txt"
echo ""

# Exit with appropriate code
if [[ $REGRESSIONS -gt 0 ]]; then
    echo -e "${RED}==============================================================${NC}"
    echo -e "${RED}BENCHMARK COMPARISON FAILED${NC}"
    echo -e "${RED}==============================================================${NC}"
    echo -e "${RED}Found $REGRESSIONS performance regressions exceeding ${THRESHOLD}%${NC}"
    echo -e "${RED}Maximum regression: ${MAX_REGRESSION}%${NC}"
    echo ""
    echo "Review the detailed comparison:"
    echo "  cat $OUTPUT_DIR/comparison.log"
    echo ""
    exit 1
else
    echo -e "${GREEN}==============================================================${NC}"
    echo -e "${GREEN}BENCHMARK COMPARISON PASSED${NC}"
    echo -e "${GREEN}==============================================================${NC}"
    echo -e "${GREEN}No significant performance regressions detected${NC}"
    echo ""
    if [[ $IMPROVEMENTS -gt 0 ]]; then
        echo -e "${GREEN}Found $IMPROVEMENTS performance improvements! 🎉${NC}"
    fi
    echo ""
    exit 0
fi

