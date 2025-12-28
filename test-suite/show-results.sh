#!/bin/bash

# Display test results with statistics and visualizations

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "╔════════════════════════════════════════════════════════════╗"
echo "║           NanoLambda Test Results Viewer                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if results directory exists
if [ ! -d "$RESULTS_DIR" ]; then
    echo "No results found. Run tests first with ./run-all-tests.sh"
    exit 1
fi

# Show latest results by default, or specific run if provided
if [ -n "$1" ]; then
    RESULT_DIR="$RESULTS_DIR/$1"
else
    RESULT_DIR="$RESULTS_DIR/latest"
fi

if [ ! -d "$RESULT_DIR" ]; then
    echo "Results not found: $RESULT_DIR"
    echo ""
    echo "Available results:"
    ls -1 "$RESULTS_DIR" | grep -v latest
    exit 1
fi

echo "Results from: $(basename $(readlink -f $RESULT_DIR))"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Display summary report
if [ -f "$RESULT_DIR/summary.md" ]; then
    cat "$RESULT_DIR/summary.md"
    echo ""
fi

# Display individual scenario results
echo "Detailed Results:"
echo "─────────────────────────────────────────────────────────────"
echo ""

for scenario_dir in "$RESULT_DIR"/*/; do
    if [ -d "$scenario_dir" ]; then
        scenario_name=$(basename "$scenario_dir")
        echo "📊 $scenario_name"
        
        if [ -f "$scenario_dir/output.log" ]; then
            # Count success/failures
            success=$(grep -c "✓" "$scenario_dir/output.log" 2>/dev/null || echo 0)
            failed=$(grep -c "✗" "$scenario_dir/output.log" 2>/dev/null || echo 0)
            
            if [ $failed -eq 0 ]; then
                echo -e "   ${GREEN}✓ All tests passed${NC} ($success successful)"
            else
                echo -e "   ${RED}✗ Some tests failed${NC} ($success passed, $failed failed)"
            fi
            
            # Show last few lines
            echo "   Last output:"
            tail -5 "$scenario_dir/output.log" | sed 's/^/   /'
        fi
        
        echo ""
    fi
done

# Display metrics if available
echo "Platform Metrics:"
echo "─────────────────────────────────────────────────────────────"

METRICS=$(curl -s http://localhost:8080/api/v1/metrics 2>/dev/null)

if [ $? -eq 0 ]; then
    echo "$METRICS" | python3 -m json.tool 2>/dev/null || echo "$METRICS"
else
    echo "Could not fetch metrics (server may not be running)"
fi

echo ""
echo "─────────────────────────────────────────────────────────────"
echo "Dashboard: http://localhost:8080/dashboard"
echo ""

# Optionally open dashboard
if [ "$2" = "--open" ]; then
    if command -v xdg-open &> /dev/null; then
        xdg-open "http://localhost:8080/dashboard"
    elif command -v open &> /dev/null; then
        open "http://localhost:8080/dashboard"
    fi
fi

# Show all available results
echo "Available Test Runs:"
echo "─────────────────────────────────────────────────────────────"
ls -lth "$RESULTS_DIR" | grep -v latest | tail -10 | awk '{print $9, "("$6, $7, $8")"}'
echo ""
echo "To view specific run: $0 <timestamp>"
echo "To open dashboard: $0 --open"
