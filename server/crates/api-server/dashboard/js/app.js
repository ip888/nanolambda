import { api } from './api.js';

const els = {
    statusPill: document.getElementById('status-pill'),
    baseUrl: document.getElementById('base-url'),
    apiKey: document.getElementById('api-key'),
    saveConnection: document.getElementById('save-connection'),
    checkHealth: document.getElementById('check-health'),
    checkMetrics: document.getElementById('check-metrics'),
    newKeyName: document.getElementById('new-key-name'),
    newKeyDays: document.getElementById('new-key-days'),
    createKey: document.getElementById('create-key'),
    refreshKeys: document.getElementById('refresh-keys'),
    keyTable: document.getElementById('key-table'),
    code: document.getElementById('code'),
    runCode: document.getElementById('run-code'),
    output: document.getElementById('output'),
};

function setStatus(message, kind = 'idle') {
    els.statusPill.textContent = message;
    els.statusPill.className = `status ${kind}`;
}

function writeOutput(label, payload) {
    const body = typeof payload === 'string' ? payload : JSON.stringify(payload, null, 2);
    els.output.textContent = `[${new Date().toLocaleTimeString()}] ${label}\n${body}`;
}

function unixFromDays(days) {
    if (!days || Number(days) <= 0) {
        return null;
    }
    return Math.floor(Date.now() / 1000) + Number(days) * 86400;
}

async function withStatus(label, fn) {
    try {
        setStatus(label, 'working');
        const result = await fn();
        setStatus('Success', 'ok');
        return result;
    } catch (error) {
        setStatus('Failed', 'error');
        writeOutput('Error', error.message || String(error));
        throw error;
    }
}

function saveConnection() {
    api.setConnection({
        baseUrl: els.baseUrl.value,
        apiKey: els.apiKey.value,
    });
    setStatus('Connection saved', 'ok');
}

async function checkHealth() {
    const health = await withStatus('Checking health...', () => api.getHealth());
    writeOutput('Health', health);
}

async function checkMetrics() {
    const metrics = await withStatus('Loading metrics...', () => api.getMetricsPrometheus());
    writeOutput('Metrics (/metrics/prometheus)', metrics.split('\n').slice(0, 40).join('\n'));
}

function renderKeys(keys) {
    if (!keys || keys.length === 0) {
        els.keyTable.innerHTML = '<p class="hint">No keys found for this account.</p>';
        return;
    }

    const rows = keys.map((k) => {
        const expires = k.expires_at ? new Date(k.expires_at * 1000).toISOString().slice(0, 10) : 'never';
        return `
            <tr>
                <td>${k.id}</td>
                <td>${k.name}</td>
                <td>${k.status}</td>
                <td>${expires}</td>
                <td><button class="btn danger" data-revoke="${k.id}">Revoke</button></td>
            </tr>
        `;
    }).join('');

    els.keyTable.innerHTML = `
        <table class="table">
            <thead>
                <tr><th>ID</th><th>Name</th><th>Status</th><th>Expires</th><th>Action</th></tr>
            </thead>
            <tbody>${rows}</tbody>
        </table>
    `;

    els.keyTable.querySelectorAll('[data-revoke]').forEach((btn) => {
        btn.addEventListener('click', async () => {
            const id = btn.getAttribute('data-revoke');
            await withStatus('Revoking key...', () => api.revokeApiKey(id));
            await refreshKeys();
        });
    });
}

async function refreshKeys() {
    const data = await withStatus('Loading keys...', () => api.listApiKeys());
    renderKeys(data.keys || []);
    writeOutput('API Keys', data);
}

async function createKey() {
    const name = (els.newKeyName.value || '').trim();
    if (!name) {
        writeOutput('Validation', 'Key name is required.');
        return;
    }
    const expiresAt = unixFromDays(els.newKeyDays.value);
    const created = await withStatus('Creating key...', () => api.createApiKey(name, expiresAt));
    if (created?.key) {
        els.apiKey.value = created.key;
        saveConnection();
    }
    writeOutput('New API Key (store this securely)', created);
    await refreshKeys();
}

async function runPython(code) {
    const result = await withStatus('Running sandbox code...', () =>
        api.invokeTool('execute_python', { code })
    );
    writeOutput('Sandbox Result', result);
}

async function runExample(kind) {
    if (kind === 'python-basic') {
        return runPython('print(2+2)');
    }
    if (kind === 'python-data') {
        return runPython('nums=[10,20,30]; print(sum(nums))');
    }
    if (kind === 'shell-basic') {
        const result = await withStatus('Running shell demo...', () =>
            api.invokeTool('execute_shell', { command: 'pwd && ls -la' })
        );
        writeOutput('Shell Result', result);
    }
}

function init() {
    els.baseUrl.value = localStorage.getItem('nl_base_url') || window.location.origin;
    els.apiKey.value = localStorage.getItem('nl_api_key') || '';
    saveConnection();

    els.saveConnection.addEventListener('click', saveConnection);
    els.checkHealth.addEventListener('click', checkHealth);
    els.checkMetrics.addEventListener('click', checkMetrics);
    els.createKey.addEventListener('click', createKey);
    els.refreshKeys.addEventListener('click', refreshKeys);
    els.runCode.addEventListener('click', () => runPython(els.code.value));

    document.querySelectorAll('[data-example]').forEach((btn) => {
        btn.addEventListener('click', () => runExample(btn.getAttribute('data-example')));
    });

    checkHealth().catch(() => {});
}

init();
