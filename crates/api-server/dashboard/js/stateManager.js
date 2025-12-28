// ============================================================
// STATE MANAGER
// ============================================================
import { api } from './api.js';
import { appState } from './state.js';
import { MetricsComponent } from './metrics.js';
import { ChartsComponent } from './charts.js';
import { InfoComponent } from './info.js';

export const StateManager = {
    async loadData(silent = false) {
        try {
            if (!silent) document.getElementById('errorContainer').style.display = 'none';
            
            // Update connection status
            this.updateConnectionStatus('connecting');
            
            const data = await api.fetchMetrics();
            appState.metrics = data;
            appState.lastUpdate = new Date();
            
            // Update UI
            this.updateAllComponents();
            this.updateLastUpdateTime();
            this.updateConnectionStatus('connected');
        } catch (error) {
            console.error('Failed to load data:', error);
            this.updateConnectionStatus('error');
            
            if (!silent) {
                document.getElementById('errorContainer').style.display = 'block';
                document.getElementById('errorText').textContent = error.message || 'Failed to connect to server';
            }
        }
    },

    updateLastUpdateTime() {
        const element = document.getElementById('lastUpdateTime');
        if (!appState.lastUpdate) return;
        
        const now = new Date();
        const diff = Math.floor((now - appState.lastUpdate) / 1000);
        
        let text;
        if (diff < 5) text = 'Just now';
        else if (diff < 60) text = `${diff}s ago`;
        else if (diff < 3600) text = `${Math.floor(diff / 60)}m ago`;
        else text = appState.lastUpdate.toLocaleTimeString();
        
        element.textContent = text;
    },

    updateConnectionStatus(status) {
        const badge = document.getElementById('statusBadge');
        const statusText = document.getElementById('statusText');
        const dot = badge.querySelector('.status-dot');
        
        badge.className = 'status-badge';
        
        switch(status) {
            case 'connecting':
                statusText.textContent = 'Connecting...';
                dot.style.background = 'var(--color-amber)';
                dot.style.boxShadow = '0 0 8px var(--color-amber)';
                badge.style.borderColor = 'var(--color-amber)';
                badge.style.background = 'rgba(245, 158, 11, 0.1)';
                break;
            case 'connected':
                statusText.textContent = 'Live';
                dot.style.background = 'var(--color-green)';
                dot.style.boxShadow = '0 0 8px var(--color-green)';
                badge.style.borderColor = 'var(--color-green)';
                badge.style.background = 'rgba(16, 185, 129, 0.1)';
                break;
            case 'error':
                statusText.textContent = 'Disconnected';
                dot.style.background = 'var(--color-red)';
                dot.style.boxShadow = '0 0 8px var(--color-red)';
                badge.style.borderColor = 'var(--color-red)';
                badge.style.background = 'rgba(239, 68, 68, 0.1)';
                break;
        }
    },

    updateAllComponents() {
        if (!appState.metrics) return;
        MetricsComponent.update(appState.metrics);
        ChartsComponent.update(appState.metrics);
        InfoComponent.update(appState.metrics);
    }
};
