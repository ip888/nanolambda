# Benchmark blog post — outline

Working title: **"We ran 10,000 agent tool calls on 6 sandbox providers. Here's the data."**

Target audience: agent framework devs (LangChain, LlamaIndex, CrewAI, Pydantic-AI users), AI-product eng leads.

## Thesis (one sentence)

*For the agent tool-call workload, NanoLambda's WASM + snapshot fast-path delivers
sub-15 ms P50 cold starts at a fraction of the per-second price, and the
experiment is reproducible from a single `cargo run`.*

## Structure

### 1. Setup (reader buys in)
- Why cold start *actually* matters for agents (serial tool-call loops → every
  cold ms compounds; Claude Sonnet 4.x tool loop does 3–8 sequential calls per
  task in prod, so 200 ms × 5 = 1 s of tail latency).
- Why price matters more than people think (cost is ~10–30% of agent product
  COGS at scale; E2B's $150/mo floor is mostly a rounding error for teams
  and a dealbreaker for indie devs).

### 2. Methodology (reader trusts)
- Hardware: Fly.io `performance-2x` for NanoLambda + identical VMs elsewhere
  where possible; AWS Lambda 128 MB x86_64.
- Four workloads: hello-world, JSON-processing, compute-heavy, I/O.
- Each platform: 1 cold start + 10 warm-up + 100 measured warm invocations +
  5 s throughput window.
- Reproducible harness: `nanolambda-benchmarks` crate in the repo, one
  command re-runs every chart.
- Non-goals: GPU sandboxes (different market), multi-tenant noisy-neighbor,
  sustained high-concurrency.

### 3. Results (charts first, words second)
- Cold start table — NanoLambda vs E2B vs AWS Lambda vs Modal (directional)
  vs Daytona (directional) vs Fly Machines (directional).
- Warm P50/P95/P99 by workload.
- Throughput per workload.
- Memory footprint (compare: NanoLambda WASM snapshot vs E2B container).
- Cost per 1M tool calls (derived from pricing + measured latency).

### 4. Why it's fast (reader respects the tech)
- Rust + wasmtime + snapshot pool + clone-on-write memory.
- Link to [`server/crates/vmm`](../server/crates/vmm).
- Honest caveat: WASM fast-path covers stateless tool calls; anything
  needing full syscalls falls back to KVM (slower, still in the report).

### 5. Pricing analysis (reader converts)
- Calculator link (see [`marketing/cost-calculator.html`](../marketing/cost-calculator.html)).
- "Free tier that isn't a trial": 50 CPU-hours real, unlimited duration.
- No Pro monthly floor; per-second billing.

### 6. Caveats (reader calibrates)
- E2B's strength is its SDK ergonomics and Firecracker microVMs — not
  included in this pass but flagged for v2.
- Modal's strength is GPU workloads — not a fair apples-to-apples here.
- Published harness is open; PRs that add platforms or workloads welcome.

### 7. CTA
- Star the repo.
- Free tier signup.
- Hiring link (if applicable).

## Distribution plan

- Hacker News (Tuesday 09:00 PT); ride the curiosity gap title.
- Cross-post to dev.to, Medium, Lobsters.
- Twitter/X thread summarizing the one-chart verdict.
- Reddit: r/LocalLLaMA, r/programming, r/MachineLearning with more neutral
  framing.
- LangChain / LlamaIndex Discord + newsletter pitch.

## Success metrics

- 5 k unique readers in week 1.
- 500 GitHub stars within 7 days of publish.
- 200 free-tier signups attributed via UTM `src=benchmark-blog`.
- ≥ 3 inbound "can I run this against X?" issues → PRs.

## Reproduce locally

```sh
cd server
cargo run --release -p nanolambda-benchmarks -- \
    --platform all --iterations 200 \
    --output ../bench-results/report.json
# Writes report.json + report.md
```
