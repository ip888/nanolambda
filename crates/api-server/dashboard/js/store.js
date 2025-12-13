// State Management Store
export function createStore() {
    return {
        // State
        apiKey: localStorage.getItem('apiKey') || '',
        metrics: null,
        usage: null,
        loading: false,
        error: null,
        modals: {
            analytics: false,
            clv: false,
            churn: false,
            paymentRetry: false
        },

        // Mutations
        setApiKey(key) {
            this.apiKey = key;
            localStorage.setItem('apiKey', key);
        },

        setMetrics(metrics) {
            this.metrics = metrics;
        },

        setUsage(usage) {
            this.usage = usage;
        },

        setLoading(loading) {
            this.loading = loading;
        },

        setError(error) {
            this.error = error;
        },

        openModal(modalName) {
            this.modals[modalName] = true;
        },

        closeModal(modalName) {
            this.modals[modalName] = false;
        },

        closeAllModals() {
            Object.keys(this.modals).forEach(key => {
                this.modals[key] = false;
            });
        }
    };
}
