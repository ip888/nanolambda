# Dashboard Architecture

## Overview

The NanoLambda dashboard has been refactored from a 2541-line monolithic HTML file into a modern, maintainable component-based architecture using Vue 3 from CDN.

## Technology Stack

- **Frontend Framework**: Vue 3 (loaded from CDN - no build step required)
- **Charts**: Chart.js for data visualization
- **Module System**: ES6 modules
- **Styling**: CSS with custom properties for theming
- **Backend**: Rust Axum serving static files via `include_bytes!` macro

## Architecture Benefits

- **Maintainability**: 41% code reduction through better organization (2541 lines → ~1500 lines across 15 files)
- **Modularity**: Each component is self-contained and testable
- **Reusability**: Components like MetricCard can be reused throughout the app
- **Separation of Concerns**: Clear boundaries between data fetching, state management, UI, and styling
- **No Build Step**: Vue 3 CDN approach means instant development feedback
- **Easy to Extend**: Adding new features is straightforward with the component system

## Directory Structure

```
crates/api-server/dashboard/
├── index.html                    # Main entry point
├── css/
│   ├── main.css                  # Core styles (layout, theming, animations)
│   └── components.css            # Component-specific styles
├── js/
│   ├── api.js                    # API client with all endpoint wrappers
│   ├── store.js                  # Centralized state management
│   ├── app.js                    # Vue app initialization
│   └── components/
│       ├── MetricCard.js         # Reusable metric display card
│       ├── StatsGrid.js          # Main metrics grid (8 cards)
│       ├── ChartsPanel.js        # Chart.js integration
│       ├── AnalyticsModal.js     # Usage analytics modal
│       ├── CLVModal.js           # Customer lifetime value modal
│       ├── ChurnModal.js         # Churn risk analysis modal
│       └── PaymentRetryModal.js  # Payment retry status modal
└── assets/                       # Future: images, fonts, etc.
```

## Component Breakdown

### Core Files

#### `index.html` (~90 lines)
- Main entry point
- Loads Vue 3 and Chart.js from CDN
- Imports all JavaScript modules
- Defines the app template structure
- Mounts Vue app to `#app` element

#### `api.js` (~120 lines)
- `ApiClient` class for centralized HTTP requests
- 20+ endpoint methods:
  - `getMetrics()` - System metrics
  - `getDashboard()` - Dashboard data
  - `getHealthScore()` - Usage analytics
  - `getChurnPrediction()` - Churn risk
  - `getCLV()` - Lifetime value
  - `getPaymentRetryStatus()` - Payment status
  - And more...
- Error handling and response parsing
- Header management (API keys)
- Singleton instance exported as `api`

#### `store.js` (~50 lines)
- `createStore()` factory function
- Centralized state management:
  - `apiKey` - User's API key (persisted to LocalStorage)
  - `metrics` - Current metrics data
  - `usage` - Usage statistics
  - `loading` - Loading state flag
  - `error` - Error message
  - `modals` - Modal visibility state
- Mutation methods: `setApiKey()`, `setMetrics()`, `setLoading()`, etc.
- Modal management: `openModal()`, `closeModal()`

#### `app.js` (~70 lines)
- Vue app initialization with `createApp()`
- Component registration
- Store initialization
- Auto-refresh every 5 seconds
- Event handlers for modal interactions
- API key persistence

### Display Components

#### `MetricCard.js` (~30 lines)
**Purpose**: Reusable card for displaying single metrics

**Props**:
- `label` - Metric name
- `value` - Primary value
- `secondary` - Optional secondary info (HTML supported)
- `icon` - Optional emoji icon
- `actions` - Array of action buttons

**Features**:
- Clean, consistent metric display
- Dynamic action button rendering
- Flexible content via slots/props

#### `StatsGrid.js` (~120 lines)
**Purpose**: Main dashboard metrics grid

**Displays**:
- Total Cost - Monthly spend
- Invocations - Total function calls
- Avg Latency - Response time with status indicator
- Cold Start Rate - Performance metric
- Success Rate - Reliability metric
- Active Connections - Current connections
- Memory Usage - Resource utilization
- Total Executions - Lifetime execution count

**Features**:
- 4 action buttons for opening modals:
  - 📊 View Analytics
  - 💎 View Lifetime Value
  - ⚠️ Churn Risk Analysis
  - 💳 Payment Retry Status
- Helper methods for status indicators:
  - `getLatencyText()` - Color codes latency (green/yellow/red)
  - `getColdStartText()` - Cold start percentage status
  - `getSuccessRateText()` - Success rate health
- Emits events to parent for modal control

#### `ChartsPanel.js` (~140 lines)
**Purpose**: Data visualization with Chart.js

**Charts**:
1. **Latency Percentiles** (Bar Chart)
   - P50, P90, P95, P99 values
   - Blue gradient color scheme
   - Tooltips with "ms" suffix

2. **Execution History** (Line Chart)
   - Time series of function executions
   - Purple line with filled area
   - Dynamic time labels

**Features**:
- Lifecycle management: `initCharts()` on mount
- Reactive updates: Watches `metrics` prop
- `updateCharts()` method for data changes
- Responsive canvas sizing

### Modal Components

#### `AnalyticsModal.js` (~180 lines)
**Purpose**: Display comprehensive usage analytics

**Data Sources**:
- Health score (0-100)
- Churn prediction
- Growth trend
- Recommendations

**Sections**:
- **Health Score**: Color-coded indicator (excellent/good/warning/critical)
- **Churn Risk**: Progress bar with risk percentage
- **Growth Metrics**: Period-over-period growth indicators
- **Recommendations**: Actionable insights with priority badges

**Features**:
- Parallel API calls for performance
- Loading state per section
- Error handling for each data source
- Color-coded health indicators

#### `CLVModal.js` (~80 lines)
**Purpose**: Display customer lifetime value

**Displays**:
- Total CLV
- Segment information (tier, category)
- Revenue forecasts:
  - 1 month
  - 6 months
  - 12 months

**Features**:
- Simplified data presentation
- Currency formatting
- Segment visualization

#### `ChurnModal.js` (~120 lines)
**Purpose**: Churn risk analysis and interventions

**Displays**:
- Risk score (0-100)
- Risk level badge (critical/high/medium/low)
- Churn probability percentage
- Days until predicted churn
- Primary risk factors with severity badges
- Top 5 intervention recommendations

**Features**:
- Color-coded risk levels:
  - Critical (70+): Red
  - High (50-69): Orange
  - Medium (30-49): Yellow
  - Low (<30): Green
- Intervention recommendations with:
  - Cost estimate
  - Timeline
  - Success rate

#### `PaymentRetryModal.js` (~130 lines)
**Purpose**: Payment retry status and history

**Displays**:
- Account status badge (active/past_due/suspended)
- Outstanding amount
- Retry attempt count
- Next scheduled retry date
- Complete retry history with:
  - Timestamps
  - Status (success/failed/pending)
  - Amounts
  - Failure reasons
- Platform-wide recovery metrics

**Features**:
- Status-based styling
- Chronological history
- Actionable insights
- Recovery rate visualization

### Styling

#### `main.css` (~200 lines)
**Core Dashboard Styles**:
- CSS custom properties (variables) for theming:
  ```css
  --bg-primary: #0f172a;
  --text-primary: #f8fafc;
  --success: #10b981;
  --warning: #f59e0b;
  --error: #ef4444;
  ```
- Header with sticky positioning
- Responsive grid layouts
- Loading spinner animation
- Chart container styles
- Error/loading state styles
- Mobile breakpoints (@media queries)

#### `components.css` (~250 lines)
**Component-Specific Styles**:
- Metric card hover effects and transitions
- Modal animations:
  - `fadeIn` (0.2s) for overlay
  - `slideUp` (0.3s) for content
- Health score color coding
- Risk level styling (badge colors)
- Priority badges (urgent/high/medium/low)
- Status badges (active/suspended/past_due)
- Retry history timeline
- Responsive component layouts

## State Management Pattern

The dashboard uses a lightweight store pattern:

```javascript
// In store.js
export function createStore() {
  return {
    // State
    apiKey: localStorage.getItem('apiKey') || '',
    metrics: null,
    loading: false,
    error: null,
    modals: { analytics: false, clv: false, churn: false, paymentRetry: false },
    
    // Mutations
    setApiKey(key) { 
      this.apiKey = key; 
      localStorage.setItem('apiKey', key); 
    },
    setMetrics(metrics) { this.metrics = metrics; },
    setLoading(loading) { this.loading = loading; },
    setError(error) { this.error = error; },
    
    // Modal Actions
    openModal(name) { this.modals[name] = true; },
    closeModal(name) { this.modals[name] = false; }
  };
}
```

## Data Flow

1. **Initialization** (app.js)
   - Vue app created with `createApp()`
   - Store initialized with `createStore()`
   - Components registered
   - Auto-refresh timer started (5s intervals)

2. **Data Fetching** (api.js → store.js)
   - User triggers refresh or auto-refresh fires
   - `fetchMetrics()` called
   - Parallel API requests via `ApiClient`
   - Store updated with new data
   - Vue reactivity triggers re-render

3. **Component Rendering** (components/*.js)
   - Components receive data via props
   - Computed properties format display values
   - User interactions emit events
   - Parent handles events (e.g., opening modals)

4. **Modal Interactions**
   - User clicks action button
   - Event emitted to parent
   - Parent calls `store.openModal(name)`
   - Modal component fetches additional data
   - Modal displays with animation
   - Close button calls `store.closeModal(name)`

## API Integration

### Endpoints Used

| Endpoint | Purpose | Component |
|----------|---------|-----------|
| `/metrics` | System metrics | StatsGrid, ChartsPanel |
| `/dashboard` | Dashboard summary | app.js |
| `/analytics/health` | Health score | AnalyticsModal |
| `/analytics/churn` | Churn prediction | AnalyticsModal, ChurnModal |
| `/analytics/growth` | Growth metrics | AnalyticsModal |
| `/analytics/recommendations` | Recommendations | AnalyticsModal |
| `/analytics/clv` | Customer lifetime value | CLVModal |
| `/payment/retry-status` | Payment retry info | PaymentRetryModal |

### Request Flow

```
User Action → Component Event → Store Method → API Client → Backend
     ↓                                                           ↓
UI Update ← Vue Reactivity ← Store Mutation ← Response ← Handler
```

## Server Configuration

### Rust Handlers (handlers.rs)

```rust
// Serve main dashboard HTML
pub async fn get_dashboard() -> Html<&'static str> {
    Html(include_str!("../dashboard/index.html"))
}

// Serve static files (JS, CSS)
pub async fn get_dashboard_file(
    axum::extract::Path(file_path): axum::extract::Path<String>,
) -> Result<(StatusCode, axum::http::HeaderMap, Vec<u8>), StatusCode> {
    // Security: block directory traversal
    if file_path.contains("..") {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Determine content type
    let content_type = if file_path.ends_with(".js") {
        "application/javascript"
    } else if file_path.ends_with(".css") {
        "text/css"
    } else {
        "text/html"
    };
    
    // Map file paths to embedded content
    let content = match file_path.as_str() {
        "js/api.js" => include_bytes!("../dashboard/js/api.js").to_vec(),
        "js/store.js" => include_bytes!("../dashboard/js/store.js").to_vec(),
        // ... all 12 files mapped
        _ => return Err(StatusCode::NOT_FOUND),
    };
    
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, content_type.parse().unwrap());
    Ok((StatusCode::OK, headers, content))
}
```

### Routes (lib.rs)

```rust
.route("/dashboard", get(handlers::get_dashboard))
.route("/dashboard/{*file}", get(handlers::get_dashboard_file))
```

**Note**: Uses Axum v0.7+ wildcard syntax `{*file}` instead of old `*file`

## Development Workflow

### Adding a New Component

1. Create component file in `dashboard/js/components/`
2. Define Vue component with `export default { ... }`
3. Import in `app.js`
4. Register in `components` object
5. Add to template in `index.html`
6. Add styles in `components.css`
7. Update `handlers.rs` to include the file in `include_bytes!` mapping
8. Rebuild project: `cargo build --release`

### Adding a New API Endpoint

1. Add method to `ApiClient` class in `api.js`
2. Use in component's data fetching logic
3. Update component to display new data
4. Test with backend endpoint

### Styling Guidelines

- Use CSS custom properties for colors
- Follow existing animation patterns
- Keep components self-contained
- Use meaningful class names (.metric-card, .modal-overlay, etc.)
- Mobile-first responsive design

## Performance Considerations

- **No Build Step**: Instant development feedback
- **CDN Loading**: Vue 3 and Chart.js loaded from CDN (cached by browsers)
- **ES6 Modules**: Browser-native module loading
- **Auto-refresh**: 5-second interval (configurable)
- **Parallel API Calls**: Multiple endpoints fetched simultaneously
- **Embedded Assets**: Static files embedded in binary via `include_bytes!` (no disk I/O)

## Browser Compatibility

- Modern browsers with ES6 module support
- Vue 3 compatible (Chrome 64+, Firefox 67+, Safari 12+, Edge 79+)
- Chart.js compatible (all modern browsers)

## Future Enhancements

1. **TypeScript Migration**: Add type safety without build step (via JSDoc or minimal tooling)
2. **Vuex/Pinia**: Upgrade to more robust state management if needed
3. **Component Tests**: Add Vitest or similar for unit testing
4. **Dark/Light Theme Toggle**: Leverage CSS custom properties for theme switching
5. **WebSocket Integration**: Real-time updates instead of polling
6. **PWA Features**: Offline support, app installation
7. **Accessibility**: ARIA labels, keyboard navigation
8. **Internationalization**: Multi-language support

## Migration from Old Dashboard

The old 2541-line monolithic `dashboard.html` has been replaced with this modular architecture.

**Backup**: Old file can be found at `crates/api-server/dashboard-old.html` (if preserved)

**Benefits of Migration**:
- 41% fewer lines of code
- Easier to maintain and debug
- Better separation of concerns
- Reusable components
- Modern development practices
- Easier to onboard new developers

## Troubleshooting

### Dashboard not loading
- Check server logs: `tail -f /workspaces/nanolambda/server.log`
- Verify server is running: `ps aux | grep nanolambda-server`
- Test endpoint: `curl http://localhost:8080/dashboard`

### JavaScript module errors
- Check browser console for 404s
- Verify file paths in `handlers.rs` match actual files
- Ensure `include_bytes!` paths are correct

### API key not persisting
- Check browser LocalStorage
- Verify `saveApiKey()` method is called
- Clear browser cache and retry

### Charts not rendering
- Check Chart.js CDN is loading
- Verify metrics data structure matches expected format
- Check browser console for Chart.js errors

## Summary

The refactored dashboard provides a modern, maintainable foundation for the NanoLambda monitoring interface. With clear component boundaries, centralized state management, and a build-free development workflow, it's easy to extend and maintain while providing a great user experience.
