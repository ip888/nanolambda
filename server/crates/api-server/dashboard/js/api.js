// API Client for NanoLambda Dashboard
export class ApiClient {
    constructor(baseUrl = '') {
        this.baseUrl = baseUrl;
    }

    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;
        const config = {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...options.headers,
            },
        };

        try {
            const response = await fetch(url, config);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            return await response.json();
        } catch (error) {
            console.error(`API request failed: ${endpoint}`, error);
            throw error;
        }
    }

    // Metrics endpoints
    async getMetrics() {
        return this.request('/metrics');
    }

    async getDashboard() {
        return this.request('/dashboard');
    }

    // Analytics endpoints
    async getHealthScore(apiKey) {
        return this.request('/analytics/health', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getChurnPrediction(apiKey) {
        return this.request('/analytics/churn-prediction', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getGrowthTrend(apiKey) {
        return this.request('/analytics/growth-trend', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getRecommendations(apiKey) {
        return this.request('/analytics/recommendations', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getPlatformAnalytics() {
        return this.request('/analytics/platform');
    }

    // CLV endpoints
    async getCLV(apiKey) {
        return this.request('/clv/calculate', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getCLVPrediction(apiKey, months) {
        return this.request(`/clv/predict?months=${months}`, {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getCLVSegment(apiKey) {
        return this.request('/clv/segment', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getCLVSegments() {
        return this.request('/clv/segments');
    }

    async getPlatformCLVSummary() {
        return this.request('/clv/summary');
    }

    // Churn endpoints
    async getChurnRisk(apiKey) {
        return this.request('/churn/risk', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getChurnMetrics() {
        return this.request('/churn/metrics');
    }

    async predictChurn() {
        return this.request('/churn/predict');
    }

    // Payment Retry endpoints
    async getPaymentRetryStatus(apiKey) {
        return this.request('/payment-retry/status', {
            headers: { 'x-api-key': apiKey }
        });
    }

    async getPaymentRetryMetrics() {
        return this.request('/payment-retry/metrics');
    }

    async processRetry(apiKey) {
        return this.request('/payment-retry/process', {
            method: 'POST',
            headers: { 'x-api-key': apiKey }
        });
    }
}

export const api = new ApiClient();
