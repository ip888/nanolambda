// Analytics Modal Component
import { api } from '../api.js';

export default {
    name: 'AnalyticsModal',
    props: {
        apiKey: String
    },
    emits: ['close'],
    data() {
        return {
            loading: true,
            data: null,
            error: null
        };
    },
    async mounted() {
        await this.fetchData();
    },
    template: `
        <div class="modal-overlay" @click.self="$emit('close')">
            <div class="modal-content">
                <div class="modal-header" style="background: linear-gradient(135deg, #06b6d4 0%, #0891b2 100%);">
                    <h2>📊 Usage Analytics</h2>
                    <button @click="$emit('close')" class="modal-close">×</button>
                </div>
                
                <div class="modal-body">
                    <div v-if="loading" class="loading">Loading analytics...</div>
                    <div v-else-if="error" class="error">{{ error }}</div>
                    <div v-else-if="data">
                        <!-- Health Score -->
                        <div class="analytics-section">
                            <h3>🏥 Health Score</h3>
                            <div class="health-score-card">
                                <div class="score-value" :class="getHealthClass(data.health?.overall_health_score)">
                                    {{ data.health?.overall_health_score || 0 }}
                                </div>
                                <div class="score-label">{{ getHealthLabel(data.health?.overall_health_score) }}</div>
                            </div>
                            <div class="metrics-grid">
                                <div class="metric-item">
                                    <span class="metric-label">Usage Frequency</span>
                                    <span class="metric-value">{{ data.health?.usage_frequency_score || 0 }}/100</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Success Rate</span>
                                    <span class="metric-value">{{ data.health?.success_rate_score || 0 }}/100</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Payment Health</span>
                                    <span class="metric-value">{{ data.health?.payment_health_score || 0 }}/100</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Engagement</span>
                                    <span class="metric-value">{{ data.health?.engagement_score || 0 }}/100</span>
                                </div>
                            </div>
                        </div>

                        <!-- Churn Prediction -->
                        <div class="analytics-section">
                            <h3>⚠️ Churn Risk</h3>
                            <div class="churn-risk-bar">
                                <div class="risk-fill" :style="{ width: (data.churn?.churn_probability * 100) + '%' }"></div>
                                <div class="risk-label">{{ (data.churn?.churn_probability * 100).toFixed(1) }}% Risk</div>
                            </div>
                            <p class="risk-assessment">{{ data.churn?.risk_level }} - {{ data.churn?.reason }}</p>
                        </div>

                        <!-- Growth Trend -->
                        <div class="analytics-section">
                            <h3>📈 Growth Trend</h3>
                            <div class="metrics-grid">
                                <div class="metric-item">
                                    <span class="metric-label">Monthly Change</span>
                                    <span class="metric-value" :class="data.growth?.trend === 'growing' ? 'positive' : 'negative'">
                                        {{ (data.growth?.growth_rate * 100).toFixed(1) }}%
                                    </span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Trend</span>
                                    <span class="metric-value">{{ data.growth?.trend }}</span>
                                </div>
                            </div>
                        </div>

                        <!-- Recommendations -->
                        <div v-if="data.recommendations?.length" class="analytics-section">
                            <h3>💡 Recommendations</h3>
                            <div v-for="rec in data.recommendations" :key="rec.action" class="recommendation-card">
                                <div class="rec-header">
                                    <span class="rec-priority" :class="'priority-' + rec.priority">{{ rec.priority }}</span>
                                    <span class="rec-category">{{ rec.category }}</span>
                                </div>
                                <div class="rec-action">{{ rec.action }}</div>
                                <div class="rec-reason">{{ rec.reason }}</div>
                                <div class="rec-impact">Impact: {{ rec.expected_impact }}</div>
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
                const [health, churn, growth, recommendations] = await Promise.all([
                    api.getHealthScore(this.apiKey),
                    api.getChurnPrediction(this.apiKey),
                    api.getGrowthTrend(this.apiKey),
                    api.getRecommendations(this.apiKey)
                ]);

                this.data = {
                    health: health.data,
                    churn: churn.data,
                    growth: growth.data,
                    recommendations: recommendations.data
                };
            } catch (error) {
                this.error = 'Failed to load analytics data';
                console.error(error);
            } finally {
                this.loading = false;
            }
        },
        getHealthClass(score) {
            if (score >= 80) return 'health-excellent';
            if (score >= 60) return 'health-good';
            if (score >= 40) return 'health-warning';
            return 'health-critical';
        },
        getHealthLabel(score) {
            if (score >= 80) return 'Excellent';
            if (score >= 60) return 'Good';
            if (score >= 40) return 'Needs Attention';
            return 'Critical';
        }
    }
};
