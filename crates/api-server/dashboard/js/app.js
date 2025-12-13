// Main Vue App
import { api } from './api.js';
import { createStore } from './store.js';
import StatsGrid from './components/StatsGrid.js';
import ChartsPanel from './components/ChartsPanel.js';
import AnalyticsModal from './components/AnalyticsModal.js';
import CLVModal from './components/CLVModal.js';
import ChurnModal from './components/ChurnModal.js';
import PaymentRetryModal from './components/PaymentRetryModal.js';

const { createApp } = Vue;

const app = createApp({
    components: {
        StatsGrid,
        ChartsPanel,
        AnalyticsModal,
        CLVModal,
        ChurnModal,
        PaymentRetryModal
    },
    data() {
        return createStore();
    },
    mounted() {
        this.fetchMetrics();
        // Auto-refresh every 5 seconds
        setInterval(() => this.fetchMetrics(), 5000);
    },
    methods: {
        async fetchMetrics() {
            this.setLoading(true);
            this.setError(null);

            try {
                const [metrics, dashboard] = await Promise.all([
                    api.getMetrics(),
                    api.getDashboard()
                ]);

                this.setMetrics(metrics);
                this.setUsage(dashboard.usage || {});
            } catch (error) {
                this.setError('Failed to fetch metrics');
                console.error('Error fetching metrics:', error);
            } finally {
                this.setLoading(false);
            }
        },
        saveApiKey() {
            this.setApiKey(this.apiKey);
            this.fetchMetrics();
        },
        openAnalyticsModal() {
            this.openModal('analytics');
        },
        openCLVModal() {
            this.openModal('clv');
        },
        openChurnModal() {
            this.openModal('churn');
        },
        openPaymentRetryModal() {
            this.openModal('paymentRetry');
        }
    }
});

app.mount('#app');
