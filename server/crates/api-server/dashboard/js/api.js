export class ApiClient {
    constructor() {
        this.baseUrl = localStorage.getItem('nl_base_url') || '';
        this.apiKey = localStorage.getItem('nl_api_key') || '';
    }

    setConnection({ baseUrl, apiKey }) {
        this.baseUrl = (baseUrl || '').trim().replace(/\/$/, '');
        this.apiKey = (apiKey || '').trim();
        localStorage.setItem('nl_base_url', this.baseUrl);
        localStorage.setItem('nl_api_key', this.apiKey);
    }

    authHeaders() {
        if (!this.apiKey) {
            return {};
        }
        return {
            Authorization: `Bearer ${this.apiKey}`,
            'x-api-key': this.apiKey,
        };
    }

    async request(endpoint, options = {}, expect = 'json') {
        const url = `${this.baseUrl}${endpoint}`;
        const headers = {
            'Content-Type': 'application/json',
            ...(options.auth === false ? {} : this.authHeaders()),
            ...(options.headers || {}),
        };

        const response = await fetch(url, {
            ...options,
            headers,
        });

        if (!response.ok) {
            let details = '';
            try {
                details = await response.text();
            } catch {
                details = '';
            }
            throw new Error(`HTTP ${response.status} ${response.statusText}${details ? ` - ${details}` : ''}`);
        }

        if (expect === 'text') {
            return response.text();
        }
        return response.json();
    }

    getHealth() {
        return this.request('/health', { method: 'GET', auth: false });
    }

    getMetricsPrometheus() {
        return this.request('/metrics/prometheus', { method: 'GET', auth: false }, 'text');
    }

    createApiKey(name, expiresAt) {
        return this.request('/auth/keys', {
            method: 'POST',
            auth: false,
            body: JSON.stringify({
                name,
                permissions: ['sandbox:invoke'],
                expires_at: expiresAt,
            }),
        });
    }

    listApiKeys() {
        return this.request('/auth/keys', { method: 'GET' });
    }

    revokeApiKey(id) {
        return this.request(`/auth/keys/${id}`, { method: 'DELETE' }, 'text');
    }

    invokeTool(tool, args) {
        return this.request('/sandbox/invoke', {
            method: 'POST',
            body: JSON.stringify({ tool, args }),
        });
    }
}

export const api = new ApiClient();
