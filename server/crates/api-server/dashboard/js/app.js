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
    playgroundLabel: document.getElementById('playground-label'),
    playgroundModeHint: document.getElementById('playground-mode-hint'),
    playgroundScenario: document.getElementById('playground-scenario'),
    runCode: document.getElementById('run-code'),
    flash: document.getElementById('flash'),
    output: document.getElementById('output'),
};

const requiredElementKeys = [
    'statusPill',
    'baseUrl',
    'apiKey',
    'saveConnection',
    'checkHealth',
    'checkMetrics',
    'newKeyName',
    'newKeyDays',
    'createKey',
    'refreshKeys',
    'keyTable',
    'code',
    'playgroundLabel',
    'playgroundModeHint',
    'playgroundScenario',
    'runCode',
    'flash',
    'output',
];

const playgroundState = {
    tool: 'execute_python',
    scenarioKey: 'custom-python',
    scenarioLabel: 'Custom Python Playground',
};

const SCENARIOS = {
    'python-simple': {
        label: 'Python Simple Scenario',
        tool: 'execute_python',
        args: {
            code: `orders = [
    {"id": "A1", "amount": 120.0, "status": "paid"},
    {"id": "A2", "amount": 80.5, "status": "paid"},
    {"id": "A3", "amount": 40.0, "status": "failed"},
    {"id": "A4", "amount": 300.0, "status": "paid"},
]

paid_total = sum(o["amount"] for o in orders if o["status"] == "paid")
failed_count = sum(1 for o in orders if o["status"] != "paid")

print("Simple Data Validation")
print("paid_total=", round(paid_total, 2))
print("failed_count=", failed_count)
print("avg_paid=", round(paid_total / (len(orders) - failed_count), 2))`
        },
        fillPlayground: true,
    },
    'python-medium': {
        label: 'Python Medium Scenario',
        tool: 'execute_python',
        args: {
            code: `from collections import defaultdict

events = [
    ("2026-01", "u1", 120), ("2026-01", "u2", 125), ("2026-01", "u3", 118),
    ("2026-02", "u1", 122), ("2026-02", "u2", 300), ("2026-02", "u4", 121),
    ("2026-03", "u1", 119), ("2026-03", "u2", 127), ("2026-03", "u3", 600),
    ("2026-03", "u4", 123),
]

month_users = defaultdict(set)
month_values = defaultdict(list)
for month, user, value in events:
    month_users[month].add(user)
    month_values[month].append(value)

months = sorted(month_users.keys())
print("Medium Cohort + Anomaly Scan")
for i, month in enumerate(months):
    users = month_users[month]
    avg = sum(month_values[month]) / len(month_values[month])
    retained = len(users.intersection(month_users[months[i-1]])) if i > 0 else len(users)
    retention = retained / len(month_users[months[i-1]]) if i > 0 else 1.0
    print(f"month={month} users={len(users)} avg={avg:.2f} retention={retention:.2%}")

all_vals = [v for _, _, v in events]
global_avg = sum(all_vals) / len(all_vals)
threshold = global_avg * 2.2
anomalies = [(m, u, v) for (m, u, v) in events if v > threshold]
print("anomaly_threshold=", round(threshold, 2))
print("anomalies=", anomalies if anomalies else "none")`
        },
        fillPlayground: true,
    },
    'python-complex': {
        label: 'Python Complex Scenario',
        tool: 'execute_python',
        args: {
            code: `import random
import time
from statistics import mean

random.seed(42)
start = time.time()

N = 25000
latencies_ms = []
failures = 0
region_stats = {"us-east": [], "eu-west": [], "ap-south": []}

for i in range(N):
    region = random.choice(list(region_stats.keys()))
    base = random.gauss(95, 20)
    penalty = 35 if random.random() < 0.07 else 0
    latency = max(5, base + penalty)
    ok = random.random() > 0.035
    if not ok:
        failures += 1
    latencies_ms.append(latency)
    region_stats[region].append(latency)

latencies_ms.sort()
p50 = latencies_ms[int(0.50 * len(latencies_ms))]
p95 = latencies_ms[int(0.95 * len(latencies_ms))]
p99 = latencies_ms[int(0.99 * len(latencies_ms))]
error_rate = failures / N

print("Complex Workload + Limits Profile")
print(f"requests={N} failures={failures} error_rate={error_rate:.2%}")
print(f"p50={p50:.2f}ms p95={p95:.2f}ms p99={p99:.2f}ms")
for region, vals in sorted(region_stats.items()):
    print(f"region={region} avg={mean(vals):.2f}ms max={max(vals):.2f}ms")

elapsed = time.time() - start
print(f"script_runtime={elapsed:.3f}s")

if p99 > 220 or error_rate > 0.06:
    print("assessment=HIGH_RISK tuning recommended")
else:
    print("assessment=PASS baseline acceptable")`
        },
        fillPlayground: true,
    },
    'shell-simple': {
        label: 'Shell Simple Scenario',
        tool: 'execute_shell',
        args: {
            command: 'echo "Shell Runtime Fingerprint" && pwd && uname -a && ls -la'
        },
    },
    'shell-medium': {
        label: 'Shell Medium Scenario',
        tool: 'execute_shell',
        args: {
            command: `set -e
mkdir -p data out
cat > data/input.csv <<'CSV'
id,team,score
1,alpha,81
2,beta,92
3,alpha,88
4,beta,95
5,gamma,77
CSV
python3 - <<'PY' > out/summary.txt
import csv
from collections import defaultdict

totals = defaultdict(float)
counts = defaultdict(int)

with open('data/input.csv', newline='') as f:
    for row in csv.DictReader(f):
        team = row['team']
        score = float(row['score'])
        totals[team] += score
        counts[team] += 1

for team in sorted(totals):
    avg = totals[team] / counts[team]
    print(f"{team},avg={avg:.2f},count={counts[team]}")
PY
echo "Medium File ETL Pipeline"
cat out/summary.txt`
        },
    },
    'shell-complex': {
        label: 'Shell Complex Scenario',
        tool: 'execute_shell',
        args: {
            command: `set -e
mkdir -p logs
python3 - <<'PY' > logs/access.log
import random

random.seed(42)
for _ in range(1200):
    subnet = random.randrange(8)
    host = random.randrange(1, 41)
    code = 200 if random.random() < 0.8 else 500
    ms = random.randrange(30, 480)
    print(f"10.0.{subnet}.{host} status={code} latency={ms}ms")
PY

echo "Complex Log Mining"
total=$(wc -l < logs/access.log)
err=$(grep -c 'status=500' logs/access.log || true)
p95=$(awk -F'latency=|ms' '{print $2}' logs/access.log | sort -n | awk '{a[NR]=$1} END {if (NR==0) {print 0} else {idx=int(NR*0.95); if (idx<1) idx=1; print a[idx]}}')
error_rate=$(awk -v e="$err" -v t="$total" 'BEGIN {if (t==0) {print "0.0000"} else {printf "%.4f", e/t}}')

echo "total=$total errors=$err error_rate=$error_rate p95_ms=$p95"
echo "top_talkers:"
awk '{print $1}' logs/access.log | sort | uniq -c | sort -nr | head -5`
        },
    },
};

function setStatus(message, kind = 'idle') {
    els.statusPill.textContent = message;
    els.statusPill.className = `status ${kind}`;
}

function setFlash(message, kind = 'ok') {
    els.flash.textContent = message;
    els.flash.className = `flash ${kind}`;
}

function writeOutput(label, payload) {
    const body = typeof payload === 'string' ? payload : JSON.stringify(payload, null, 2);
    els.output.textContent = `[${new Date().toLocaleTimeString()}] ${label}\n${body}`;
}

function writeSandboxOutput(label, payload) {
    const stdout = ((payload && payload.stdout) || '').replace(/\r\n/g, '\n').trimEnd();
    const stderr = ((payload && payload.stderr) || '').replace(/\r\n/g, '\n').trimEnd();
    const exitCode = payload && Number.isFinite(payload.exit_code) ? payload.exit_code : 'n/a';
    const durationMs = payload && Number.isFinite(payload.duration_ms) ? payload.duration_ms : 'n/a';
    const coldStart = payload && typeof payload.cold_start === 'boolean' ? payload.cold_start : 'n/a';

    const lines = [
        `[${new Date().toLocaleTimeString()}] ${label}`,
        `Mode: ${playgroundState.tool === 'execute_shell' ? 'Shell' : 'Python'}`,
        `Exit Code: ${exitCode} | Duration: ${durationMs} ms | Cold Start: ${coldStart}`,
        '',
        'STDOUT:',
        stdout || '(empty)',
        '',
        'STDERR:',
        stderr || '(empty)',
    ];

    els.output.textContent = lines.join('\n');
}

function setPlaygroundMode(tool, scenarioKey, scenarioLabel) {
    const isShell = tool === 'execute_shell';
    playgroundState.tool = tool;
    playgroundState.scenarioKey = scenarioKey;
    playgroundState.scenarioLabel = scenarioLabel;

    els.playgroundLabel.textContent = isShell ? 'Shell Command' : 'Python Code';
    els.playgroundModeHint.textContent = `Current mode: ${isShell ? 'Shell' : 'Python'}`;
    els.playgroundScenario.textContent = `Scenario: ${scenarioLabel}`;
    els.runCode.textContent = isShell ? 'Run Shell' : 'Run Python';
    els.runCode.classList.toggle('shell-mode', isShell);

    document.querySelectorAll('[data-example]').forEach((btn) => {
        btn.classList.toggle('active', btn.getAttribute('data-example') === scenarioKey);
    });
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
        setFlash(label, 'working');
        const result = await fn();
        setStatus('Success', 'ok');
        setFlash('Done.', 'ok');
        return result;
    } catch (error) {
        setStatus('Failed', 'error');
        setFlash(error.message || String(error), 'error');
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
    setFlash('Connection saved.', 'ok');
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
    let name = (els.newKeyName.value || '').trim();
    if (!name) {
        name = `dashboard-key-${new Date().toISOString().replace(/[:.]/g, '-')}`;
        els.newKeyName.value = name;
        setFlash('Key name was empty. Auto-generated a name.', 'working');
    }
    const expiresAt = unixFromDays(els.newKeyDays.value);
    const created = await withStatus('Creating key...', () => api.createApiKey(name, expiresAt));
    if (created?.key) {
        els.apiKey.value = created.key;
        saveConnection();
        setFlash('Key created and saved to connection.', 'ok');
    }
    writeOutput('New API Key (store this securely)', created);
    await refreshKeys();
}

async function runPython(code) {
    const result = await withStatus('Running Python in sandbox...', () =>
        api.invokeTool('execute_python', { code })
    );
    writeSandboxOutput('Sandbox Result', result);
}

async function runShell(command) {
    const result = await withStatus('Running Shell in sandbox...', () =>
        api.invokeTool('execute_shell', { command })
    );
    writeSandboxOutput('Sandbox Result', result);
}

async function runPlayground() {
    const script = (els.code.value || '').trim();
    if (!script) {
        setFlash('Playground is empty. Add Python code or shell command first.', 'error');
        return;
    }

    if (playgroundState.tool === 'execute_shell') {
        await runShell(script);
    } else {
        await runPython(script);
    }
}

async function runExample(kind) {
    const scenario = SCENARIOS[kind];
    if (!scenario) {
        setFlash(`Unknown scenario: ${kind}`, 'error');
        return;
    }

    const script = scenario.tool === 'execute_shell' ? scenario.args.command : scenario.args.code;
    els.code.value = script;
    setPlaygroundMode(scenario.tool, kind, scenario.label);
    setFlash(`Loaded ${scenario.label}. Edit in Playground and click ${scenario.tool === 'execute_shell' ? 'Run Shell' : 'Run Python'}.`, 'ok');
    writeOutput(
        'Scenario Loaded',
        `${scenario.label} loaded into Playground. You can now modify it to prove output is not hardcoded before execution.`
    );
}

function init() {
    const missing = requiredElementKeys.filter((k) => !els[k]);
    if (missing.length > 0) {
        const message = `Dashboard wiring error: missing elements [${missing.join(', ')}].`;
        console.error(message);
        if (els.output) {
            els.output.textContent = message;
        }
        return;
    }

    els.baseUrl.value = localStorage.getItem('nl_base_url') || window.location.origin;
    els.apiKey.value = localStorage.getItem('nl_api_key') || '';
    if (!els.newKeyName.value.trim()) {
        els.newKeyName.value = 'customer-demo-key';
    }
    saveConnection();

    els.saveConnection.addEventListener('click', saveConnection);
    els.checkHealth.addEventListener('click', checkHealth);
    els.checkMetrics.addEventListener('click', checkMetrics);
    els.createKey.addEventListener('click', createKey);
    els.refreshKeys.addEventListener('click', refreshKeys);
    els.runCode.addEventListener('click', runPlayground);
    els.newKeyName.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
            event.preventDefault();
            createKey();
        }
    });

    document.querySelectorAll('[data-example]').forEach((btn) => {
        btn.addEventListener('click', () => runExample(btn.getAttribute('data-example')));
    });

    setPlaygroundMode('execute_python', 'custom-python', 'Custom Python Playground');

    checkHealth().catch(() => {});
}

init();
