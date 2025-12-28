#!/bin/bash

# Cleanup test data and functions

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║           NanoLambda Test Cleanup                         ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

ACTION=${1:-prompt}

cleanup_functions() {
    echo "Cleaning up test functions..."
    
    # Get API key
    if [ -z "$API_KEY" ]; then
        API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
            -H "Content-Type: application/json" \
            -d '{"name": "cleanup", "expires_at": null}' | \
            grep -o '"key":"[^"]*' | cut -d'"' -f4)
    fi
    
    # List all functions
    FUNCTIONS=$(curl -s http://localhost:8080/functions \
        -H "Authorization: Bearer $API_KEY" | \
        grep -o '"name":"[^"]*' | cut -d'"' -f4)
    
    # Delete test functions
    deleted=0
    for func in $FUNCTIONS; do
        if [[ $func == test-* ]] || [[ $func == stress-* ]] || [[ $func == demo-* ]]; then
            curl -s -X DELETE "http://localhost:8080/functions/$func" \
                -H "Authorization: Bearer $API_KEY" > /dev/null
            echo "  ✓ Deleted function: $func"
            deleted=$((deleted + 1))
        fi
    done
    
    echo "✓ Deleted $deleted test functions"
}

cleanup_results() {
    echo "Cleaning up old test results..."
    
    if [ -d "$SCRIPT_DIR/results" ]; then
        # Keep only the last 5 test runs
        cd "$SCRIPT_DIR/results"
        ls -t | grep -v latest | tail -n +6 | xargs -r rm -rf
        
        count=$(ls -1 | grep -v latest | wc -l)
        echo "✓ Kept last 5 test runs ($count total)"
    else
        echo "  No results directory found"
    fi
}

cleanup_temp() {
    echo "Cleaning up temporary files..."
    
    # Remove temp function packages
    rm -f /tmp/function_*.zip
    rm -rf /tmp/stress-test/
    rm -rf /tmp/test-*/
    
    echo "✓ Cleaned temporary files"
}

stop_monitoring() {
    echo "Stopping continuous monitoring..."
    
    if [ -f "$SCRIPT_DIR/.continuous-monitor.pid" ]; then
        PID=$(cat "$SCRIPT_DIR/.continuous-monitor.pid")
        kill $PID 2>/dev/null || true
        rm -f "$SCRIPT_DIR/.continuous-monitor.pid"
        echo "✓ Stopped monitoring (PID: $PID)"
    else
        echo "  No monitoring process running"
    fi
}

reset_database() {
    echo "Resetting database (WARNING: This deletes ALL data)..."
    
    if [ -f "/tmp/nanolambda.db" ]; then
        rm -f "/tmp/nanolambda.db"
        echo "✓ Database reset"
        echo "  Restart the server to recreate the database"
    else
        echo "  No database file found"
    fi
}

# Interactive mode
if [ "$ACTION" = "prompt" ]; then
    echo "Select cleanup options:"
    echo "  1) Test functions only"
    echo "  2) Old test results only"
    echo "  3) Temporary files only"
    echo "  4) Stop monitoring"
    echo "  5) All of the above"
    echo "  6) FULL RESET (includes database)"
    echo ""
    read -p "Enter choice [1-6]: " choice
    
    case $choice in
        1)
            cleanup_functions
            ;;
        2)
            cleanup_results
            ;;
        3)
            cleanup_temp
            ;;
        4)
            stop_monitoring
            ;;
        5)
            cleanup_functions
            cleanup_results
            cleanup_temp
            stop_monitoring
            ;;
        6)
            read -p "Are you sure? This will delete ALL data (y/N): " confirm
            if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
                stop_monitoring
                cleanup_functions
                cleanup_results
                cleanup_temp
                reset_database
            else
                echo "Cancelled"
                exit 0
            fi
            ;;
        *)
            echo "Invalid choice"
            exit 1
            ;;
    esac
elif [ "$ACTION" = "--functions" ]; then
    cleanup_functions
elif [ "$ACTION" = "--results" ]; then
    cleanup_results
elif [ "$ACTION" = "--temp" ]; then
    cleanup_temp
elif [ "$ACTION" = "--monitoring" ]; then
    stop_monitoring
elif [ "$ACTION" = "--all" ]; then
    cleanup_functions
    cleanup_results
    cleanup_temp
    stop_monitoring
elif [ "$ACTION" = "--full-reset" ]; then
    read -p "Delete ALL data including database? (y/N): " confirm
    if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
        stop_monitoring
        cleanup_functions
        cleanup_results
        cleanup_temp
        reset_database
    else
        echo "Cancelled"
        exit 0
    fi
else
    echo "Usage: $0 [option]"
    echo ""
    echo "Options:"
    echo "  (no option)      - Interactive mode"
    echo "  --functions      - Delete test functions"
    echo "  --results        - Delete old test results"
    echo "  --temp           - Delete temporary files"
    echo "  --monitoring     - Stop monitoring"
    echo "  --all            - All of the above"
    echo "  --full-reset     - Full reset including database"
    exit 1
fi

echo ""
echo "✓ Cleanup complete"
