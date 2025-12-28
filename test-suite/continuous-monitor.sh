#!/bin/bash

# 24/7 Continuous Monitoring and Testing
# Runs perpetual load tests with health monitoring

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/.continuous-monitor.pid"
LOG_FILE="$SCRIPT_DIR/results/continuous-monitor.log"
STATE_FILE="$SCRIPT_DIR/results/monitor-state.json"

ACTION=${1:-start}

start_monitoring() {
    if [ -f "$PID_FILE" ]; then
        echo "Monitor already running (PID: $(cat $PID_FILE))"
        exit 1
    fi
    
    echo "Starting continuous monitoring..."
    
    nohup bash -c '
        CYCLE=0
        START_TIME=$(date +%s)
        
        while true; do
            CYCLE=$((CYCLE + 1))
            CURRENT_TIME=$(date +%s)
            UPTIME=$((CURRENT_TIME - START_TIME))
            
            echo "═══════════════════════════════════════════════════════════"
            echo "Cycle #$CYCLE - Uptime: $(($UPTIME / 3600))h $(($UPTIME % 3600 / 60))m"
            echo "═══════════════════════════════════════════════════════════"
            
            # Health check
            if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
                echo "ERROR: Server health check failed!"
                exit 1
            fi
            
            # Get current metrics
            METRICS=$(curl -s http://localhost:8080/api/v1/metrics)
            INVOCATIONS=$(echo "$METRICS" | grep -o "\"total_invocations\":[0-9]*" | cut -d: -f2)
            AVG_LATENCY=$(echo "$METRICS" | grep -o "\"avg_latency_ms\":[0-9.]*" | cut -d: -f2)
            
            echo "Current stats: $INVOCATIONS invocations, ${AVG_LATENCY}ms avg latency"
            
            # Save state
            cat > "'$STATE_FILE'" << EOF
{
    "cycle": $CYCLE,
    "uptime_seconds": $UPTIME,
    "total_invocations": ${INVOCATIONS:-0},
    "avg_latency_ms": ${AVG_LATENCY:-0},
    "last_update": "$(date -Iseconds)"
}
EOF
            
            # Run random test pattern
            PATTERN=$((RANDOM % 4))
            
            case $PATTERN in
                0)
                    echo "Pattern: Light load (10 req/min)"
                    for i in {1..10}; do
                        bash "'$SCRIPT_DIR'/run-tests.sh" python sanity > /dev/null 2>&1 &
                        sleep 6
                    done
                    ;;
                1)
                    echo "Pattern: Medium load (60 req/min)"
                    for i in {1..60}; do
                        bash "'$SCRIPT_DIR'/run-tests.sh" nodejs sanity > /dev/null 2>&1 &
                        sleep 1
                    done
                    ;;
                2)
                    echo "Pattern: Burst load (100 concurrent)"
                    for i in {1..100}; do
                        bash "'$SCRIPT_DIR'/run-tests.sh" python sanity > /dev/null 2>&1 &
                    done
                    wait
                    ;;
                3)
                    echo "Pattern: Mixed languages"
                    bash "'$SCRIPT_DIR'/run-tests.sh" mixed integration > /dev/null 2>&1
                    ;;
            esac
            
            # Wait before next cycle (5 minutes)
            sleep 300
        done
    ' > "$LOG_FILE" 2>&1 &
    
    echo $! > "$PID_FILE"
    echo "✓ Continuous monitoring started (PID: $!)"
    echo "  - Log: $LOG_FILE"
    echo "  - State: $STATE_FILE"
    echo "  - Dashboard: http://localhost:8080/dashboard"
    echo ""
    echo "To stop: $0 stop"
}

stop_monitoring() {
    if [ ! -f "$PID_FILE" ]; then
        echo "Monitor not running"
        exit 1
    fi
    
    PID=$(cat "$PID_FILE")
    echo "Stopping continuous monitoring (PID: $PID)..."
    
    kill $PID 2>/dev/null || true
    rm -f "$PID_FILE"
    
    echo "✓ Monitor stopped"
}

status_monitoring() {
    if [ ! -f "$PID_FILE" ]; then
        echo "Status: NOT RUNNING"
        exit 1
    fi
    
    PID=$(cat "$PID_FILE")
    
    if ps -p $PID > /dev/null 2>&1; then
        echo "Status: RUNNING (PID: $PID)"
        
        if [ -f "$STATE_FILE" ]; then
            echo ""
            echo "Current State:"
            cat "$STATE_FILE"
        fi
        
        echo ""
        echo "Recent Activity (last 20 lines):"
        tail -20 "$LOG_FILE"
    else
        echo "Status: STOPPED (stale PID file)"
        rm -f "$PID_FILE"
    fi
}

case $ACTION in
    start)
        start_monitoring
        ;;
    stop)
        stop_monitoring
        ;;
    restart)
        stop_monitoring 2>/dev/null || true
        sleep 2
        start_monitoring
        ;;
    status)
        status_monitoring
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        exit 1
        ;;
esac
