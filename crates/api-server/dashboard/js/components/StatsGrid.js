// Stats Grid Component
import MetricCard from './MetricCard.js';

export default {
    name: 'StatsGrid',
    components: { MetricCard },
    props: {
        metrics: Object,
        usage: Object
    },
    emits: ['open-analytics', 'open-clv', 'open-churn', 'open-payment-retry'],
    computed: {
        hasUsage() {
            return this.usage && Object.keys(this.usage).length > 0;
        },
        totalCostActions() {
            return [
                {
                    label: '📊 View Analytics',
                    handler: () => this.$emit('open-analytics'),
                    style: 'background: linear-gradient(135deg, #06b6d4 0%, #0891b2 100%);'
                },
                {
                    label: '💎 View Lifetime Value',
                    handler: () => this.$emit('open-clv'),
                    style: 'background: linear-gradient(135deg, #10b981 0%, #059669 100%);'
                },
                {
                    label: '⚠️ Churn Risk Analysis',
                    handler: () => this.$emit('open-churn'),
                    style: 'background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);'
                },
                {
                    label: '💳 Payment Retry Status',
                    handler: () => this.$emit('open-payment-retry'),
                    style: 'background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);'
                }
            ];
        }
    },
    template: `
        <div class="stats-grid">
            <metric-card
                icon="💰"
                label="Total Cost"
                :value="hasUsage ? '$' + ((usage.total_cost_cents || 0) / 100).toFixed(2) : '$0.00'"
                :secondary="hasUsage ? 
                    '🕒 ' + (usage.total_duration_ms || 0).toLocaleString() + 'ms total<br>' +
                    '💾 ' + ((usage.total_memory_mb || 0) / 1024).toFixed(2) + 'GB used' : 
                    ''"
                :actions="totalCostActions"
            />

            <metric-card
                icon="📊"
                label="Total Invocations"
                :value="hasUsage ? (usage.total_invocations || 0).toLocaleString() : '0'"
                :secondary="hasUsage ?
                    '✅ ' + (usage.successful_invocations || 0) + ' success<br>' +
                    '❌ ' + (usage.failed_invocations || 0) + ' failed' :
                    ''"
            />

            <metric-card
                icon="⚡"
                label="Avg Latency"
                :value="(metrics?.avg_latency_ms || 0) + 'ms'"
                :secondary="getLatencyText(metrics?.avg_latency_ms || 0)"
            />

            <metric-card
                icon="❄️"
                label="Cold Start Rate"
                :value="((metrics?.cold_start_rate || 0) * 100).toFixed(1) + '%'"
                :secondary="getColdStartText((metrics?.cold_start_rate || 0) * 100)"
            />

            <metric-card
                icon="📈"
                label="Success Rate"
                :value="((metrics?.success_rate || 0) * 100).toFixed(1) + '%'"
                :secondary="getSuccessRateText((metrics?.success_rate || 0) * 100)"
            />

            <metric-card
                icon="👥"
                label="Active Connections"
                :value="(metrics?.active_connections || 0).toString()"
            />

            <metric-card
                icon="💾"
                label="Memory Usage"
                :value="(metrics?.memory_usage_mb || 0) + 'MB'"
            />

            <metric-card
                icon="🎯"
                label="Total Executions"
                :value="(metrics?.total_executions || 0).toLocaleString()"
            />
        </div>
    `,
    methods: {
        getLatencyText(latency) {
            if (latency < 100) return '⚡ Excellent';
            if (latency < 500) return '⚠️ Acceptable';
            return '❌ Slow';
        },
        getColdStartText(rate) {
            if (rate < 10) return '⚡ Excellent';
            if (rate < 30) return '✅ Good';
            return '⚠️ High';
        },
        getSuccessRateText(rate) {
            if (rate >= 99) return '⚡ Excellent';
            if (rate >= 95) return '✅ Good';
            return '⚠️ Needs attention';
        }
    }
};
