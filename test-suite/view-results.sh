#!/bin/bash

# Simple script to view test results from the database

echo "╔════════════════════════════════════════════════════════════╗"
echo "║           NanoLambda Test Results Viewer                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

DB="/workspaces/nanolambda/nanolambda.db"

if [ ! -f "$DB" ]; then
    echo "❌ Database not found at $DB"
    exit 1
fi

echo "📊 OVERALL STATISTICS"
echo "═══════════════════════════════════════════════════════════"
sqlite3 "$DB" -column -header \
    "SELECT 'Total Functions' as Metric, COUNT(*) as Value FROM functions
     UNION ALL SELECT 'Total Invocations', COUNT(*) FROM invocations
     UNION ALL SELECT 'Successful', COUNT(*) FROM invocations WHERE status='success'
     UNION ALL SELECT 'Failed', COUNT(*) FROM invocations WHERE status='error'
     UNION ALL SELECT 'Success Rate %', ROUND(100.0 * COUNT(CASE WHEN status='success' THEN 1 END) / COUNT(*), 1) FROM invocations;"

echo ""
echo "📈 BY FUNCTION"
echo "═══════════════════════════════════════════════════════════"
sqlite3 "$DB" -column -header \
    "SELECT f.name as Function, 
            i.status as Status,
            COUNT(*) as Count,
            f.runtime as Runtime
     FROM invocations i
     JOIN functions f ON i.function_id = f.id
     GROUP BY f.name, i.status, f.runtime
     ORDER BY Count DESC;"

echo ""
echo "🕐 RECENT ACTIVITY (Last 10 invocations)"
echo "═══════════════════════════════════════════════════════════"
sqlite3 "$DB" -column -header \
    "SELECT f.name as Function,
            i.status as Status,
            f.runtime as Runtime,
            datetime(i.started_at, 'unixepoch') as Time
     FROM invocations i
     JOIN functions f ON i.function_id = f.id
     ORDER BY i.started_at DESC 
     LIMIT 10;"

echo ""
echo "📁 DATABASE INFO"
echo "═══════════════════════════════════════════════════════════"
ls -lh "$DB" | awk '{print "Size: " $5}'
echo "Location: $DB"

echo ""
echo "🌐 VIEW IN DASHBOARD"
echo "═══════════════════════════════════════════════════════════"
echo "Open: http://localhost:8080/dashboard"
echo "Metrics API: curl http://localhost:8080/metrics"

echo ""
