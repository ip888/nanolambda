// CLV Modal Component
import { api } from '../api.js';

export default {
    name: 'CLVModal',
    props: { apiKey: String },
    emits: ['close'],
    data() {
        return { loading: true, data: null, error: null };
    },
    async mounted() {
        await this.fetchData();
    },
    template: `
        <div class="modal-overlay" @click.self="$emit('close')">
            <div class="modal-content">
                <div class="modal-header" style="background: linear-gradient(135deg, #10b981 0%, #059669 100%);">
                    <h2>💎 Customer Lifetime Value</h2>
                    <button @click="$emit('close')" class="modal-close">×</button>
                </div>
                <div class="modal-body">
                    <div v-if="loading" class="loading">Loading CLV data...</div>
                    <div v-else-if="error" class="error">{{ error }}</div>
                    <div v-else-if="data">
                        <div class="clv-summary">
                            <div class="clv-value">\${{ (data.clv?.total_clv_cents / 100).toFixed(2) }}</div>
                            <div class="clv-label">Predicted Lifetime Value</div>
                        </div>
                        <div class="metrics-grid">
                            <div class="metric-item">
                                <span class="metric-label">Segment</span>
                                <span class="metric-value">{{ data.segment?.segment_name }}</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">1-Month Forecast</span>
                                <span class="metric-value">\${{ (data.clv?.predicted_revenue_1_month_cents / 100).toFixed(2) }}</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">6-Month Forecast</span>
                                <span class="metric-value">\${{ (data.clv?.predicted_revenue_6_month_cents / 100).toFixed(2) }}</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">12-Month Forecast</span>
                                <span class="metric-value">\${{ (data.clv?.predicted_revenue_12_month_cents / 100).toFixed(2) }}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    `,
    methods: {
        async fetchData() {
            if (!this.apiKey) {
                this.error = 'Please set your API key first';
                this.loading = false;
                return;
            }
            try {
                const [clv, segment] = await Promise.all([
                    api.getCLV(this.apiKey),
                    api.getCLVSegment(this.apiKey)
                ]);
                this.data = { clv: clv.data, segment: segment.data };
            } catch (error) {
                this.error = 'Failed to load CLV data';
                console.error(error);
            } finally {
                this.loading = false;
            }
        }
    }
};
