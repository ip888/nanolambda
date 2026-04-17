// ============================================================
// COMPONENT: INFO CARDS
// ============================================================
export const InfoComponent = {
    cards: [
        {
            id: 'system',
            title: 'System Info',
            icon: 'fa-server',
            items: [
                { label: 'Status', value: 'Healthy', status: 'success' },
                { label: 'Version', value: '0.1.0' },
                { label: 'Runtime', value: 'Rust/Axum' },
                { label: 'Database', value: 'SQLite', status: 'success' }
            ]
        },
        {
            id: 'performance',
            title: 'Performance',
            icon: 'fa-rocket',
            items: [
                { label: 'P50 Latency', valueKey: 'all_time.p50_latency_ms', unit: 'ms' },
                { label: 'P95 Latency', valueKey: 'all_time.p95_latency_ms', unit: 'ms' },
                { label: 'P99 Latency', valueKey: 'all_time.p99_latency_ms', unit: 'ms' },
                { label: 'Timeouts', valueKey: 'all_time.timeouts' }
            ]
        },
        {
            id: 'reliability',
            title: 'Reliability',
            icon: 'fa-shield-alt',
            items: [
                { label: 'Uptime', value: '99.9%', status: 'success' },
                { label: 'SLA Status', value: 'Active', status: 'success' },
                { label: 'Last Incident', value: 'None' },
                { label: 'MTTR', value: 'N/A' }
            ]
        }
    ],

    render() {
        const container = document.getElementById('infoContainer');
        container.innerHTML = this.cards.map(card => `
            <div class="info-card">
                <div class="component-header">
                    <div class="component-title">
                        <i class="fas ${card.icon}"></i> ${card.title}
                    </div>
                </div>
                <div id="info-${card.id}"></div>
            </div>
        `).join('');
    },

    update(data) {
        this.cards.forEach(card => {
            const container = document.getElementById(`info-${card.id}`);
            if (!container) return;
            
            // If container is empty, do initial render
            if (container.children.length === 0) {
                const html = card.items.map(item => {
                    const value = item.valueKey 
                        ? this.getNestedValue(data, item.valueKey) + (item.unit || '')
                        : item.value;
                    const statusClass = item.status ? ` ${item.status}` : '';
                    return `
                        <div class="info-item">
                            <span class="info-label">${item.label}</span>
                            <span class="info-value${statusClass}" data-label="${item.label}">${value}</span>
                        </div>
                    `;
                }).join('');
                container.innerHTML = html;
            } else {
                // Update values in-place
                card.items.forEach(item => {
                    const valueEl = container.querySelector(`[data-label="${item.label}"]`);
                    if (!valueEl) return;
                    
                    const newValue = item.valueKey 
                        ? this.getNestedValue(data, item.valueKey) + (item.unit || '')
                        : item.value;
                    
                    if (valueEl.textContent !== newValue) {
                        valueEl.classList.add('updating');
                        valueEl.textContent = newValue;
                        setTimeout(() => valueEl.classList.remove('updating'), 300);
                    }
                });
            }
        });
    },

    getNestedValue(obj, path) {
        return path.split('.').reduce((curr, prop) => curr?.[prop], obj);
    }
};
