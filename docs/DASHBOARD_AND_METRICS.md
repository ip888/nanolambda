# Dashboard and Metrics Guide

## Data Source: Real vs Sample Data

### Current State
The dashboard shows **SAMPLE/DEMO DATA** when there are no real invocations recorded.

### How Data Flows

```
Function Invocation → Runtime Records Metrics → Storage Saves to SQLite → API /metrics endpoint → Dashboard Display
```

### Getting Real Metrics

**Step 1: Create API Key**
```bash
curl -X POST http://localhost:8080/api/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "my-key"}'
```

**Step 2: Deploy a Function**
```bash
# Python example
curl -X POST http://localhost:8080/api/functions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_KEY_HERE" \
  -d '{
    "name": "hello",
    "runtime": "python",
    "handler": "main.handler",
    "code": "def handler(event, context):\n    return {\"message\": \"Hello World\"}",
    "memory_mb": 128,
    "timeout_ms": 3000
  }'
```

**Step 3: Invoke Function**
```bash
curl -X POST http://localhost:8080/api/functions/hello/invoke \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_KEY_HERE" \
  -d '{"name": "User"}'
```

**Step 4: Check Dashboard**
Now refresh the dashboard - you'll see REAL metrics:
- Total invocations: 1
- Latency: actual execution time
- Cold start: true (first invocation)

### Why Sample Data Shows

The dashboard JavaScript checks if `value === 0`:
```javascript
if (num === 0 && metric.id === 'invocations') num = 1234; // Sample
```

Once you have real invocations, real numbers replace sample data automatically!

### Metrics Database Location

All metrics stored in: `/tmp/nanolambda.db` (SQLite)

**Tables:**
- `metrics` - individual invocation records
- Aggregated on-the-fly by API endpoint

---

## Resource Consumption Analysis

### Actual Usage (from `ps aux`)

| Component | CPU | Memory | Notes |
|-----------|-----|--------|-------|
| nanolambda-server | 0% | 11 MB | Extremely efficient! |
| Dashboard | 0% | N/A | Static HTML/CSS/JS |
| Communication | negligible | N/A | JSON over HTTP |

**Conclusion:** NanoLambda uses virtually NO resources. VS Code extensions (Rust Analyzer ~3GB RAM) consume most resources.

### Why So Efficient?

1. **Rust** - Zero-cost abstractions, no GC
2. **Axum** - Async, non-blocking I/O
3. **SQLite** - In-process, no network overhead
4. **Static Dashboard** - No framework overhead

---

## Dashboard Refresh Behavior

### Chart Refresh Buttons
- ✅ **Fixed**: Icon spins (not entire button)
- Uses `icon.classList.add('loading')` 
- 500ms animation duration

### Main Refresh Button  
- ✅ **Fixed**: Silent refresh mode
- No screen flashing
- Updates values in-place
- Smooth color transitions (300ms)

### Auto-Refresh
- Every 5 seconds (configurable)
- Silent mode (no error popups)
- Updates "Last Update" timestamp
- Connection status indicator

---

## Monitoring Dashboard Features

### Production-Quality Features

1. **Real-time Status**
   - Live connection indicator (Green/Yellow/Red)
   - Last update timestamp ("Just now", "5s ago")
   - Pulsing status dot

2. **Error Handling**
   - Retry button on errors
   - Silent background refresh
   - Auto-recovery on connection restore

3. **Smooth Updates**
   - Values flash briefly on change (blue highlight)
   - Charts update without redraw
   - No DOM manipulation overhead

4. **Performance Optimized**
   - Compact layout
   - Minimal chart size (120px height)
   - Fewer ticks, smaller fonts
   - No chart animations

---

## Troubleshooting

### Dashboard Shows All Zeros
**Cause:** No functions invoked yet
**Fix:** Deploy and invoke a function (see steps above)

### Dashboard Not Loading
**Cause:** Server not running
**Fix:** 
```bash
/workspaces/nanolambda/target/release/nanolambda-server &
```

### Old Data Displayed
**Cause:** Browser cache
**Fix:** Hard refresh (Ctrl+Shift+R or Cmd+Shift+R)

---

## Configuration

### Refresh Intervals (in index.html)
```javascript
const CONFIG = {
    refresh: {
        metrics: 5000,  // 5 seconds
        charts: 10000,  // 10 seconds  
        summary: 3000   // 3 seconds
    }
};
```

### Chart Settings
- Height: 120px (compact)
- Max ticks: 4 (Y-axis), 6 (X-axis)
- Point radius: 2px (hover: 4px)
- No legend (saves space)

---

## Next Steps

1. Generate real metrics by invoking functions
2. Monitor performance in production
3. Add custom metrics as needed
4. Scale horizontally (multiple server instances)
