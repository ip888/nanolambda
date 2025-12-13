// Payment Retry Modal Component
import { api } from '../api.js';

export default {
    name: 'PaymentRetryModal',
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
                <div class="modal-header" style="background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);">
                    <h2>💳 Payment Retry Status</h2>
                    <button @click="$emit('close')" class="modal-close">×</button>
                </div>
                <div class="modal-body">
                    <div v-if="loading" class="loading">Loading payment data...</div>
                    <div v-else-if="error" class="error">{{ error }}</div>
                    <div v-else-if="data">
                        <div v-if="data.status && data.status.account_status !== 'active'" class="payment-status">
                            <div class="status-badge" :class="'status-' + data.status.account_status">
                                {{ data.status.account_status.toUpperCase() }}
                            </div>
                            <div class="metrics-grid">
                                <div class="metric-item">
                                    <span class="metric-label">Outstanding Amount</span>
                                    <span class="metric-value">\${{ (data.status.outstanding_amount_cents / 100).toFixed(2) }}</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Retry Attempt</span>
                                    <span class="metric-value">{{ data.status.current_attempt }} / {{ data.status.max_attempts }}</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Next Retry</span>
                                    <span class="metric-value">
                                        {{ data.status.next_retry_at ? new Date(data.status.next_retry_at).toLocaleDateString() : 'No more retries' }}
                                    </span>
                                </div>
                            </div>
                            <div v-if="data.status.retry_history?.length" class="retry-history">
                                <h3>Retry History</h3>
                                <div v-for="attempt in data.status.retry_history" :key="attempt.attempt_number" class="retry-attempt">
                                    <div class="attempt-header">
                                        <span>Attempt #{{ attempt.attempt_number }}</span>
                                        <span :class="'status-' + attempt.status">{{ attempt.status }}</span>
                                    </div>
                                    <div class="attempt-details">
                                        \${{ (attempt.amount_cents / 100).toFixed(2) }} on {{ new Date(attempt.attempted_at).toLocaleDateString() }}
                                        <span v-if="attempt.failure_reason"> - {{ attempt.failure_reason }}</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div v-else class="no-issues">
                            <div class="success-icon">✅</div>
                            <h3>No Payment Issues</h3>
                            <p>All payments are current. No retries needed.</p>
                        </div>
                        <div v-if="data.metrics" class="analytics-section">
                            <h3>Platform Metrics</h3>
                            <div class="metrics-grid">
                                <div class="metric-item">
                                    <span class="metric-label">Failed Payments</span>
                                    <span class="metric-value">{{ data.metrics.total_failed_payments }}</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">In Retry</span>
                                    <span class="metric-value">{{ data.metrics.accounts_in_retry }}</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Recovery Rate</span>
                                    <span class="metric-value">{{ data.metrics.recovery_rate_percentage.toFixed(1) }}%</span>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">Total Outstanding</span>
                                    <span class="metric-value">\${{ (data.metrics.total_outstanding_cents / 100).toFixed(2) }}</span>
                                </div>
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
                const [status, metrics] = await Promise.all([
                    api.getPaymentRetryStatus(this.apiKey),
                    api.getPaymentRetryMetrics()
                ]);
                this.data = {
                    status: status.data,
                    metrics: metrics.data
                };
            } catch (error) {
                this.error = 'Failed to load payment data';
                console.error(error);
            } finally {
                this.loading = false;
            }
        }
    }
};
