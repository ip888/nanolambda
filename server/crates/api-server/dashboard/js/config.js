// ============================================================
// CONFIGURATION
// ============================================================
export const CONFIG = {
    api: {
        baseUrl: 'http://localhost:8080',
        endpoints: {
            metrics: '/metrics'
        }
    },
    refresh: {
        metrics: 5000,      // 5 seconds
        charts: 10000,      // 10 seconds
        summary: 3000       // 3 seconds
    },
    colors: {
        blue: '#3b82f6',
        green: '#10b981',
        amber: '#f59e0b',
        red: '#ef4444'
    }
};
