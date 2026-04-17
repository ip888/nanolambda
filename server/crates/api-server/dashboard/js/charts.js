// ============================================================
// COMPONENT: CHARTS
// ============================================================
import { CONFIG } from './config.js';
import { appState } from './state.js';
import { StateManager } from './stateManager.js';

export const ChartsComponent = {
    charts: [
        { id: 'invocationChart', title: 'Invocation Trend', icon: 'fa-chart-line', type: 'line' },
        { id: 'latencyChart', title: 'Latency Distribution', icon: 'fa-chart-bar', type: 'bar' }
    ],

    render() {
        const container = document.getElementById('chartsContainer');
        container.innerHTML = this.charts.map(chart => `
            <div class="component chart-component">
                <div class="component-header">
                    <div class="component-title">
                        <i class="fas ${chart.icon}"></i> ${chart.title}
                    </div>
                    <div class="component-controls">
                        <button class="refresh-btn" data-chart="${chart.id}">
                            <i class="fas fa-sync-alt"></i>
                        </button>
                    </div>
                </div>
                <div class="chart-container">
                    <canvas id="${chart.id}"></canvas>
                </div>
            </div>
        `).join('');

        // Attach refresh listeners
        document.querySelectorAll('[data-chart]').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                const button = e.currentTarget;
                button.classList.add('loading');
                
                // Simulate chart refresh
                await new Promise(resolve => setTimeout(resolve, 500));
                await StateManager.loadData(true);
                
                button.classList.remove('loading');
            });
        });
    },

    update(data) {
        const hourly = data.hourly || [];
        // If no hourly data, create sample data for demonstration
        const hasData = hourly.length > 0;
        const labels = hasData ? hourly.map((_, i) => `${i}h ago`).reverse() : ['23h', '20h', '17h', '14h', '11h', '8h', '5h', '2h', 'now'];
        const invocations = hasData ? hourly.map(d => d.invocations || 0).reverse() : [12, 18, 15, 22, 19, 25, 20, 28, 24];
        const latencies = hasData ? hourly.map(d => d.avg_latency_ms || 0).reverse() : [8, 12, 9, 11, 10, 13, 11, 14, 12];

        this.updateOrCreateChart('invocationChart', {
            type: 'line',
            labels,
            datasets: [{
                label: 'Invocations',
                data: invocations,
                borderColor: CONFIG.colors.blue,
                backgroundColor: 'rgba(59, 130, 246, 0.05)',
                borderWidth: 1.5,
                fill: true,
                tension: 0.4,
                pointRadius: 2,
                pointHoverRadius: 4,
                pointBackgroundColor: CONFIG.colors.blue
            }]
        });

        this.updateOrCreateChart('latencyChart', {
            type: 'bar',
            labels,
            datasets: [{
                label: 'Latency (ms)',
                data: latencies,
                backgroundColor: CONFIG.colors.green,
                borderRadius: 4
            }]
        });
    },

    updateOrCreateChart(chartId, config) {
        const ctx = document.getElementById(chartId);
        if (!ctx) return;

        // If chart exists, update data smoothly without destroying
        if (appState.chartInstances[chartId]) {
            const chart = appState.chartInstances[chartId];
            chart.data.labels = config.labels;
            chart.data.datasets = config.datasets;
            chart.update('none'); // Update without animation for smooth feel
            return;
        }

        // Create new chart only if it doesn't exist
        appState.chartInstances[chartId] = new Chart(ctx, {
            type: config.type,
            data: {
                labels: config.labels,
                datasets: config.datasets
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                animation: false,
                interaction: {
                    mode: 'index',
                    intersect: false
                },
                plugins: {
                    legend: {
                        display: false
                    }
                },
                scales: {
                    y: {
                        beginAtZero: true,
                        grid: { 
                            color: '#334155',
                            drawBorder: false
                        },
                        ticks: { 
                            color: '#94a3b8',
                            font: { size: 9 },
                            maxTicksLimit: 4
                        }
                    },
                    x: {
                        grid: { display: false },
                        ticks: { 
                            color: '#94a3b8',
                            font: { size: 9 },
                            maxRotation: 0,
                            autoSkip: true,
                            maxTicksLimit: 6
                        }
                    }
                }
            }
        });
    }
};
