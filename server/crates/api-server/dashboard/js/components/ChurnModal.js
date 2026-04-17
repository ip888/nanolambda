// Churn Modal Component
import { api } from '../api.js';

export default {
    name: 'ChurnModal',
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
                <div class="modal-header" style="background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);">
                    <h2>⚠️ Churn Risk Analysis</h2>
                    <button @click="$emit('close')" class="modal-close">×</button>
                </div>
                <div class="modal-body">
                    <div v-if="loading" class="loading">Loading churn data...</div>
                    <div v-else-if="error" class="error">{{ error }}</div>
                    <div v-else-if="data && data.risk">
                        <div class="risk-summary">
                            <div class="risk-score" :class="getRiskClass(data.risk.risk_score)">
                                {{ data.risk.risk_score }}
                            </div>
                            <div class="risk-label">{{ data.risk.risk_level }} Risk</div>
                            <div class="risk-details">
                                {{ (data.risk.likelihood_to_churn * 100).toFixed(0) }}% churn probability
                                <span v-if="data.risk.days_until_predicted_churn">
                                    in {{ data.risk.days_until_predicted_churn }} days
                                </span>
                            </div>
                        </div>
                        <div v-if="data.risk.primary_risk_factors?.length" class="analytics-section">
                            <h3>Risk Factors</h3>
                            <div v-for="factor in data.risk.primary_risk_factors" :key="factor.factor" class="risk-factor">
                                <div class="factor-header">
                                    <span class="factor-name">{{ factor.factor }}</span>
                                    <span class="factor-severity" :class="'severity-' + factor.severity">{{ factor.severity }}</span>
                                </div>
                                <div class="factor-description">{{ factor.description }}</div>
                                <div class="factor-impact">Impact: {{ factor.impact_score }}/100</div>
                            </div>
                        </div>
                        <div v-if="data.risk.intervention_recommendations?.length" class="analytics-section">
                            <h3>Recommended Interventions</h3>
                            <div v-for="(intervention, idx) in data.risk.intervention_recommendations.slice(0, 5)" :key="idx" class="intervention-card">
                                <div class="intervention-header">
                                    <span class="intervention-priority" :class="'priority-' + intervention.priority">{{ intervention.priority }}</span>
                                    <span class="intervention-success">{{ (intervention.expected_success_rate * 100).toFixed(0) }}% success rate</span>
                                </div>
                                <div class="intervention-action">{{ intervention.action }}</div>
                                <div class="intervention-details">
                                    Cost: \${{ (intervention.estimated_cost / 100).toFixed(2) }} | 
                                    Timeline: {{ intervention.timeline }} | 
                                    Owner: {{ intervention.owner }}
                                </div>
                            </div>
                        </div>
                    </div>
                    <div v-else class="no-data">No churn analysis data available</div>
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
                const risk = await api.getChurnRisk(this.apiKey);
                this.data = { risk: risk.data };
            } catch (error) {
                this.error = 'No churn analysis available yet';
                console.error(error);
            } finally {
                this.loading = false;
            }
        },
        getRiskClass(score) {
            if (score >= 70) return 'risk-critical';
            if (score >= 50) return 'risk-high';
            if (score >= 30) return 'risk-medium';
            return 'risk-low';
        }
    }
};
