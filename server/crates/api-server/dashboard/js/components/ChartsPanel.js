// Charts Panel Component
export default {
    name: 'ChartsPanel',
    props: {
        metrics: Object
    },
    data() {
        return {
            latencyChart: null,
            executionsChart: null
        };
    },
    mounted() {
        this.initCharts();
        this.updateCharts();
    },
    watch: {
        metrics: {
            handler() {
                this.updateCharts();
            },
            deep: true
        }
    },
    template: `
        <div class="charts-panel">
            <div class="chart-container">
                <h3>⚡ Latency Percentiles</h3>
                <canvas id="latency-chart"></canvas>
            </div>
            <div class="chart-container">
                <h3>📊 Execution History</h3>
                <canvas id="executions-chart"></canvas>
            </div>
        </div>
    `,
    methods: {
        initCharts() {
            const latencyCtx = document.getElementById('latency-chart');
            const executionsCtx = document.getElementById('executions-chart');

            if (latencyCtx) {
                this.latencyChart = new Chart(latencyCtx, {
                    type: 'bar',
                    data: {
                        labels: ['P50', 'P90', 'P95', 'P99'],
                        datasets: [{
                            label: 'Latency (ms)',
                            data: [0, 0, 0, 0],
                            backgroundColor: [
                                'rgba(16, 185, 129, 0.8)',
                                'rgba(59, 130, 246, 0.8)',
                                'rgba(251, 146, 60, 0.8)',
                                'rgba(239, 68, 68, 0.8)'
                            ]
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            y: {
                                beginAtZero: true,
                                ticks: { color: '#94a3b8' },
                                grid: { color: '#334155' }
                            },
                            x: {
                                ticks: { color: '#94a3b8' },
                                grid: { color: '#334155' }
                            }
                        }
                    }
                });
            }

            if (executionsCtx) {
                this.executionsChart = new Chart(executionsCtx, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [{
                            label: 'Executions',
                            data: [],
                            borderColor: 'rgb(59, 130, 246)',
                            backgroundColor: 'rgba(59, 130, 246, 0.1)',
                            tension: 0.4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            y: {
                                beginAtZero: true,
                                ticks: { color: '#94a3b8' },
                                grid: { color: '#334155' }
                            },
                            x: {
                                ticks: { color: '#94a3b8' },
                                grid: { color: '#334155' }
                            }
                        }
                    }
                });
            }
        },
        updateCharts() {
            if (!this.metrics) return;

            // Update latency chart
            if (this.latencyChart) {
                this.latencyChart.data.datasets[0].data = [
                    this.metrics.p50_latency_ms || 0,
                    this.metrics.p90_latency_ms || 0,
                    this.metrics.p95_latency_ms || 0,
                    this.metrics.p99_latency_ms || 0
                ];
                this.latencyChart.update();
            }

            // Update executions chart (mock time series data)
            if (this.executionsChart) {
                const now = new Date();
                const labels = [];
                const data = [];
                for (let i = 11; i >= 0; i--) {
                    const time = new Date(now - i * 5000);
                    labels.push(time.toLocaleTimeString());
                    data.push(Math.floor(Math.random() * 100));
                }
                this.executionsChart.data.labels = labels;
                this.executionsChart.data.datasets[0].data = data;
                this.executionsChart.update();
            }
        }
    }
};
