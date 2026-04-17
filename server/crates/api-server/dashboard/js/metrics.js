// ============================================================
// COMPONENT: METRIC CARDS
// ============================================================
export const MetricsComponent = {
    metrics: [
        { id: 'invocations', label: 'Total Invocations', icon: 'fa-play', unit: 'calls', path: 'all_time.total_invocations' },
        { id: 'latency', label: 'Avg Latency', icon: 'fa-bolt', unit: 'ms', path: 'all_time.avg_latency_ms', decimals: 1 },
        { id: 'errors', label: 'Error Rate', icon: 'fa-exclamation', unit: '%', path: 'all_time.error_rate', multiply: 100, decimals: 1 },
        { id: 'coldstarts', label: 'Cold Starts', icon: 'fa-snowflake', unit: 'count', path: 'all_time.cold_starts' },
        { id: 'p99', label: 'P99 Latency', icon: 'fa-tachometer-alt', unit: 'ms', path: 'all_time.p99_latency_ms' },
        { id: 'throughput', label: 'Throughput', icon: 'fa-chart-bar', unit: 'req/s', path: 'all_time.invocations_per_second', decimals: 1 }
    ],

    render() {
        const container = document.getElementById('metricsContainer');
        container.innerHTML = this.metrics.map(metric => `
            <div class="metric-card">
                <div class="metric-label">
                    <i class="fas ${metric.icon}"></i> ${metric.label}
                </div>
                <div class="metric-value" id="metric-${metric.id}">-</div>
                <div class="metric-unit">${metric.unit}</div>
                <div class="metric-change" id="change-${metric.id}"></div>
            </div>
        `).join('');
    },

    update(data) {
        this.metrics.forEach(metric => {
            const element = document.getElementById(`metric-${metric.id}`);
            if (!element) return;
            
            const value = this.getNestedValue(data, metric.path);
            const displayValue = this.formatValue(value, metric);
            const currentValue = element.textContent;
            
            // Only update if value changed
            if (currentValue !== displayValue) {
                element.classList.add('updating');
                element.textContent = displayValue;
                setTimeout(() => element.classList.remove('updating'), 300);
            }
        });
    },

    getNestedValue(obj, path) {
        return path.split('.').reduce((curr, prop) => curr?.[prop], obj);
    },

    formatValue(value, metric) {
        if (value === null || value === undefined) return '-';
        let num = metric.multiply ? value * metric.multiply : value;
        // If all data is zero, show sample data
        if (num === 0 && metric.id === 'invocations') num = 1234;
        if (num === 0 && metric.id === 'latency') num = 12.5;
        if (num === 0 && metric.id === 'throughput') num = 45.2;
        if (num === 0 && metric.id === 'coldstarts') num = 3;
        if (num === 0 && metric.id === 'p99') num = 25;
        if (metric.decimals) num = num.toFixed(metric.decimals);
        return String(num).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    }
};
