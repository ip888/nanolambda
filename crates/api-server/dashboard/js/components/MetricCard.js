// Metric Card Component
export default {
    name: 'MetricCard',
    props: {
        label: String,
        value: [String, Number],
        secondary: String,
        icon: String,
        actions: Array
    },
    template: `
        <div class="metric-card">
            <div class="metric-label">{{ icon }} {{ label }}</div>
            <div class="metric-value">{{ value }}</div>
            <div v-if="secondary" class="metric-secondary" v-html="secondary"></div>
            <div v-if="actions && actions.length" class="metric-actions">
                <button 
                    v-for="action in actions" 
                    :key="action.label"
                    @click="action.handler"
                    :style="action.style"
                    class="action-button"
                >
                    {{ action.label }}
                </button>
            </div>
        </div>
    `
};
