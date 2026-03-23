# 0D Master API - Agent Development Instructions

## Project Overview

**0D Master API** is a Rust vault aggregator API for the [0D Finance](https://app.0d.finance) platform. It aggregates data from multiple vault backends (Jaffar, Vesu), indexes Starknet blockchain events, calculates user KPIs (PnL, Sharpe, Sortino, drawdown), and exposes a unified REST API.

- **Language**: Rust (edition 2024, toolchain 1.89.0)
- **Framework**: Axum 0.8 (async Tokio)
- **ORM**: Diesel 2.1 (async, with deadpool connection pooling)
- **Database**: PostgreSQL 17 + TimescaleDB 2.20.1
- **Observability**: OpenTelemetry → Alloy → Grafana LGTM stack

**Live endpoints**:
- Production: `https://api.0d.finance/v1`
- Swagger UI: `https://api.0d.finance/v1/docs`

---

## Repository Structure

```
0d-master-api/
├── bin/0d-bin/              # Binary entry point (main.rs, cli.rs)
├── crates/
│   ├── 0d-api/              # HTTP handlers, router, middleware, DTOs, errors
│   ├── 0d-db/               # Diesel models, schema, migrations, pool
│   ├── 0d-indexer/           # Starknet blockchain event indexer (Apibara)
│   ├── 0d-kpi/              # KPI engine (PnL, Sharpe, Sortino, drawdown)
│   ├── 0d-master/           # Vault API clients (Jaffar, Vesu)
│   ├── 0d-quoting/          # Pyth oracle pricing
│   └── 0d-types/            # Shared types (Currency enum, etc.)
├── contracts/               # Starknet smart contracts (Scarb)
├── helm/                    # Kubernetes Helm chart
├── migrations → crates/0d-db/migrations/
├── Dockerfile               # Multi-stage build (cargo-chef)
├── cloudbuild.yaml          # GCP Cloud Build (dev)
├── cloudbuild.prod.yaml     # GCP Cloud Build (prod)
├── docker-compose.dev.yaml  # Local dev (Postgres + LGTM)
├── diesel.toml              # Diesel CLI config
└── Makefile                 # fmt + clippy checks
```

---

## Prerequisites

### Tools Required

| Tool | Purpose | Install |
|------|---------|---------|
| **Rust 1.89.0+** | Build the project | `rustup install 1.89.0` |
| **Rust nightly** | Formatting (`cargo +nightly fmt`) | `rustup install nightly` |
| **Docker & Docker Compose** | Local dev environment (Postgres, LGTM) | [docker.com](https://docs.docker.com/get-docker/) |
| **diesel_cli** | Database migrations | `cargo install diesel_cli --no-default-features --features postgres` |
| **cargo-nextest** | Test runner | `cargo install cargo-nextest` |
| **gcloud CLI** | GCP access, Cloud Build, Artifact Registry | `brew install google-cloud-sdk` then `gcloud auth login` |
| **kubectl** | Kubernetes cluster access | `gcloud components install kubectl` |
| **Helm** | Kubernetes package manager | `brew install helm` |
| **ArgoCD CLI** (optional) | ArgoCD management | `brew install argocd` |

### GCP / Kubernetes Access

```bash
# Authenticate with GCP
gcloud auth login
gcloud config set project prod-pragma

# Get GKE cluster credentials (europe-west1)
gcloud container clusters get-credentials <cluster-name> --region europe-west1 --project prod-pragma

# Verify access
kubectl get namespaces | grep mainnet
```

### Repositories Needed

| Repo | Path | Branch | Purpose |
|------|------|--------|---------|
| **0d-master-api** | `~/Documents/GitHub/0d-master-api` | `main` | Application source code + Helm chart |
| **devops** | `~/Documents/GitHub/devops` | `prod` | ArgoCD apps, Helm values, DB config |
| **api-clients** | `~/Documents/GitHub/api-clients` | `main` | OpenAPI specs + generated Rust SDKs |

---

## Local Development Setup

### Start Local Environment

```bash
# Start PostgreSQL 17 + Grafana LGTM stack
docker compose -f docker-compose.dev.yaml up -d

# Run database migrations (auto-runs on app startup too)
diesel migration run

# Build and run
cargo run --bin 0d-bin
```

### Environment Variables

Copy from `.env.example`:

```env
DATABASE_URL=postgresql://pragma_user:pragma_password@localhost:5432/pragma
DATABASE_MAX_CONN=45
API_PORT=4242
CORS_ALLOWED_ORIGINS=http://localhost:3000,https://app.0d.finance
OTEL_COLLECTOR_ENDPOINT=http://localhost:4317
APIBARA_API_KEY=<your_apibara_key>
```

### Code Quality

```bash
# Format (MUST use nightly)
cargo +nightly fmt

# Lint
cargo clippy --locked --all-targets --all-features -- -D warnings --no-deps

# Test
cargo nextest run
```

---

## Database

### Connection

**Production** (PostgreSQL 17 + TimescaleDB, managed by CloudNativePG operator):
- **Host**: `zd-postgres-cluster-rw.mainnet` (in-cluster DNS)
- **Database**: `pragma_0d`
- **User**: `pragma_0d`
- **Extensions**: `uuid-ossp`, `timescaledb_toolkit`
- **Pooler**: PgBouncer (session mode, max 50 clients, pool size 25)

The full `DATABASE_URL` is set as a plaintext env var in the Helm values (see Deployment section).

To get database credentials from k8s secrets:
```bash
# App credentials
kubectl get secret zd-postgres-cluster-app -n mainnet -o jsonpath='{.data.uri}' | base64 -d

# Superuser credentials
kubectl get secret zd-postgres-cluster-superuser -n mainnet -o jsonpath='{.data.uri}' | base64 -d
```

### Schema (Core Tables)

| Table | Purpose |
|-------|---------|
| `vaults` | Vault registry: metadata, fees, constraints, API endpoints, status |
| `users` | User registry indexed from Starknet |
| `user_positions` | Current positions: share_balance, cost_basis |
| `user_transactions` | All txns: deposit, withdraw, fee, rebalance (with tx_hash, block_number) |
| `user_kpis` | Cached metrics: PnL, Sharpe ratio, Sortino ratio, drawdown |
| `user_portfolio_history` | Historical portfolio snapshots |
| `indexer_state` | Indexer progress tracking per vault (last block, sync status) |

- All monetary values use `DECIMAL(36,18)` (Ethereum standard precision)
- Migrations live in `crates/0d-db/migrations/` and auto-run at startup via `embed_migrations!()`

### Adding Migrations

```bash
diesel migration generate <migration_name>
# Edit the generated up.sql / down.sql in crates/0d-db/migrations/
diesel migration run
# Schema auto-updates in crates/0d-db/src/schema.rs
```

---

## API Structure

### Routes

All routes are prefixed with `/v1`. Defined in `crates/0d-api/src/router.rs`.

**Vault endpoints** (`/v1/vaults`):
- `GET /` — List all vaults
- `GET /{vault_id}` — Vault details
- `GET /{vault_id}/stats` — Statistics
- `GET /{vault_id}/timeseries` — Historical data
- `GET /{vault_id}/kpis` — Performance KPIs
- `GET /{vault_id}/liquidity` — Liquidity info
- `GET /{vault_id}/liquidity/curve` — Slippage curve
- `POST /{vault_id}/liquidity/simulate` — Simulate deposit/withdrawal
- `GET /{vault_id}/apr/summary` — APR summary
- `GET /{vault_id}/apr/series` — APR time series
- `GET /{vault_id}/composition` — Current composition
- `GET /{vault_id}/composition/series` — Composition history
- `GET /{vault_id}/caps` — Deposit/withdrawal caps
- `GET /{vault_id}/nav/latest` — Latest NAV
- `GET /{vault_id}/info` — Vault information

**User endpoints** (`/v1/users`):
- `GET /{address}` — User profile
- `GET /{address}/redeems` — Pending redeems
- `GET /{address}/vaults/{vault_id}/summary` — Position summary
- `GET /{address}/vaults/{vault_id}/transactions` — Transaction history
- `GET /{address}/vaults/{vault_id}/kpis` — User KPIs
- `GET /{address}/vaults/{vault_id}/historical` — Performance history

**Infra**:
- `GET /health` — Health check
- `GET /v1/docs` — Swagger UI
- `GET /v1/docs/openapi.json` — OpenAPI spec

### Middleware Stack (innermost → outermost)

1. **OpenTelemetry Tracing** — distributed tracing with context propagation
2. **Rate Limiting** — IP-based via `tower_governor` (configurable per-second + burst)
3. **Request Timeout** — default 30s (`REQUEST_TIMEOUT_SECS`)
4. **CORS** — configurable origins from `CORS_ALLOWED_ORIGINS`

### Data Aggregation Pattern

Most vault endpoints follow this pattern:
1. Fetch vault metadata from database
2. Call vault's backend API (Jaffar or Vesu SDK) for live data
3. Merge and return combined response

The vault backend clients are in `crates/0d-master/src/clients/` and implement the `VaultMasterClient` trait.

---

## Vault Backend SDKs (api-clients)

The repo at `../api-clients` is the source of truth for vault API specs and generated Rust SDKs.

### How It Works

1. OpenAPI specs for Jaffar and Vesu are mirrored in `api-clients/apis/<provider>/<version>/openapi.json`
2. Rust SDKs are auto-generated via `cargo-progenitor` → `api-clients/clients/rust/<provider>-sdk/`
3. SDKs are published to the Pragma private registry (`sparse+https://registry.production.pragma.build/`)
4. CI runs daily (weekdays 9AM UTC) or on `repository_dispatch` from upstream repos

### Updating an SDK

If you need to update a vault backend client:
```bash
cd ../api-clients
just fetch <provider> <version> <openapi_url>   # Mirror new spec
just diff <provider>                              # Check for breaking changes
just gen <provider> <version> <new_semver>        # Generate new SDK
just publish                                      # Publish to Pragma registry
```

Then bump the dependency version in `0d-master-api/crates/0d-master/Cargo.toml`.

### Available SDKs

| SDK | API | Key Endpoints |
|-----|-----|---------------|
| `jaffar-sdk` | 0D capital allocation | `/v1/master/apr/*`, `/v1/master/composition`, `/v1/master/kpis`, `/v1/master/nav/*`, `/v1/vault/*` |
| `vesu-sdk` | Vault event indexing | `/vaults`, `/vaults/{addr}/history`, `/user/{addr}/positions`, `/user/{addr}/redeems` |

---

## Deployment

### Architecture

```
GitHub Push/Tag
       │
       ▼
GCP Cloud Build ──► Docker Image ──► GCP Artifact Registry
                                            │
                                            ▼
                                    ArgoCD (auto-sync)
                                            │
                                            ▼
                                    GKE Cluster (mainnet namespace)
                                    ├── Deployment (1 replica)
                                    ├── Service (ClusterIP:3000)
                                    ├── Ingress (api.0d.finance)
                                    └── CNPG PostgreSQL + PgBouncer
```

### Build Pipeline

**Dev** (`cloudbuild.yaml`): Triggered on every push, builds image tagged `latest` + `$SHORT_SHA` on E2_HIGHCPU_8.

**Prod** (`cloudbuild.prod.yaml`): Triggered on git tag, builds image tagged `$TAG_NAME` on E2_HIGHCPU_32.

**Image registry**: `europe-west1-docker.pkg.dev/prod-pragma/production-docker-repo/0d-master-api-prod`

### Deploying a New Version

The deployment is a two-repo process: build from `0d-master-api`, deploy via `devops`.

1. **Tag a release** in the `0d-master-api` repo:
   ```bash
   cd ~/Documents/GitHub/0d-master-api
   git tag v0.1.22
   git push origin v0.1.22
   ```
2. **GCP Cloud Build** picks up the tag and builds + pushes `0d-master-api-prod:v0.1.22`
3. **Update the Helm values** in the `devops` repo (`prod` branch):
   ```bash
   cd ~/Documents/Github/devops
   git checkout prod
   # Edit mainnet/0d-api-service/resources/values-mainnet-0d-master-api.yaml
   # Change: tag: "v0.1.22"
   git commit -am "deploy 0d-master-api v0.1.22"
   git push origin prod
   ```
4. **ArgoCD** detects the push to `prod` branch and auto-syncs (prune + selfHeal enabled)
5. **Verify**:
   ```bash
   # Check ArgoCD sync status
   kubectl get application mainnet-0d-api-service -n argocd
   # Or via ArgoCD CLI
   argocd app get mainnet-0d-api-service

   # Check pod is running new version
   kubectl get pods -n mainnet -l app.kubernetes.io/instance=mainnet-0d-master-api-service -o wide
   ```

### Updating Environment Variables or Config

Environment variables and all deployment config live in the **devops repo**, NOT in the application repo:

```bash
cd ~/Documents/Github/devops && git checkout prod

# App-specific config (image tag, env vars, ingress, resources):
#   mainnet/0d-api-service/resources/values-mainnet-0d-master-api.yaml
#
# Shared config (security context, service account, node placement):
#   mainnet/0d-api-service/resources/values-common.yaml
#
# Database config (CNPG cluster, PgBouncer, storage):
#   mainnet/0d-api-service/resources/db.yaml
#
# ArgoCD application definition:
#   mainnet/0d-api-service/argo-app.yaml

# After editing, push to trigger ArgoCD sync:
git commit -am "update 0d config: <description>"
git push origin prod
```

### Kubernetes Resources

**Namespace**: `mainnet`

| Resource | Name | Details |
|----------|------|---------|
| Deployment | `mainnet-0d-master-api-service` | 1 replica, 50m-100m CPU, 64-128Mi RAM |
| Service | `mainnet-0d-master-api-service` | ClusterIP, port 3000 |
| Ingress | `0d-master-api` | nginx class, host `api.0d.finance`, TLS |
| PDB | `mainnet-0d-master-api-service` | minAvailable: 1 |
| DB Cluster | `zd-postgres-cluster` | CNPG TimescaleDB, 1 instance, 2Gi storage |
| DB Pooler | `zd-postgres-cluster-rw` | PgBouncer, session mode, max 50 clients |

**Node placement**: `Nodelabel: node-mainnet` with toleration `mainnet=reserved:NoSchedule`
**Service account**: `pragma-publisher-sa`
**Security**: non-root (UID 10001), read-only filesystem

### ArgoCD Application

**Name**: `mainnet-0d-api-service` (namespace: `argocd`)

**Sources** (multi-source):
1. `astraly-labs/devops` branch `prod` → Helm values
2. `cloudnative-pg.github.io/charts` chart `cluster` v0.2.1 → DB operator
3. `astraly-labs/0d-master-api` branch `HEAD` path `helm` → App Helm chart

**Sync policy**: Automated with prune + selfHeal

### Key Files in devops Repo

```
~/Documents/Github/devops/  (branch: prod)
└── mainnet/0d-api-service/
    ├── argo-app.yaml                          # ArgoCD Application definition
    └── resources/
        ├── values-common.yaml                 # Shared Helm values (security, SA, node placement)
        ├── values-mainnet-0d-master-api.yaml  # App-specific values (image tag, env vars, ingress)
        └── db.yaml                            # CNPG TimescaleDB cluster config
```

---

## Credentials & Secrets

### Production Credentials

| Credential | How to Access | Notes |
|------------|--------------|-------|
| **Database URL** | In Helm values (`values-mainnet-0d-master-api.yaml`) or `kubectl describe deploy -n mainnet` | Currently plaintext in env vars |
| **DB app secret** | `kubectl get secret zd-postgres-cluster-app -n mainnet` | Keys: `uri`, `user`, `password`, `host`, `port`, `dbname` |
| **DB superuser** | `kubectl get secret zd-postgres-cluster-superuser -n mainnet` | Same keys as above |
| **Apibara API key** | In Helm values or pod env | `APIBARA_API_KEY` — for blockchain indexing |
| **GCP SA** | `kubectl get secret google-service-account-secret -n mainnet` | Mounted at `/var/secrets/google/service-account.json` |
| **TLS cert** | `kubectl get secret production-zd-finance-certs -n mainnet` | Wildcard `*.0d.finance`, auto-managed by cert-manager |
| **Pragma registry token** | GitHub Actions secret `PRAGMA_TOKEN` in api-clients repo | For publishing SDKs to Pragma registry |

### Accessing the Database Directly

```bash
# Port-forward to the database
kubectl port-forward svc/zd-postgres-cluster-rw -n mainnet 5432:5432

# Connect (get password from secret)
export PGPASSWORD=$(kubectl get secret zd-postgres-cluster-app -n mainnet -o jsonpath='{.data.password}' | base64 -d)
psql -h localhost -U pragma_0d -d pragma_0d

# Or port-forward to the PgBouncer pooler
kubectl port-forward svc/zd-postgres-cluster-pooler-rw -n mainnet 6432:5432
```

---

## Observability

- **Traces/Logs/Metrics**: Sent via OTLP to `http://alloy.lgtm.svc.cluster.local:4317` (Grafana Alloy)
- **Dashboard**: Grafana (check dashboards for `0d-master-api-service`)
- **Pod annotation**: `instrumentation.opentelemetry.io/inject-sdk: "true"` enables auto-instrumentation

### Checking Logs

```bash
# Live pod logs
kubectl logs -f deploy/mainnet-0d-master-api-service -n mainnet

# Or via Grafana Loki with label filter: {app="mainnet-0d-master-api-service"}
```

---

## Coding Conventions

### Rust Style
- Use idiomatic Rust patterns (see [Rust Unofficial Patterns](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html))
- Use `rust_decimal` constants: `Decimal::TWO`, `Decimal::ONE_HUNDRED`, etc.
- Error types with `thiserror` crate
- Tests with `rstest` fixtures/cases
- Always run `cargo +nightly fmt` before committing

### Logging
- Use bracketed context with emoji: `[🚨 Indexer]`, `[📊 KPI]`, `[⚡ API]`
- Keep logs consistent, not spammy

### File Organization
- If a file gets too large, split into a module folder: `module/mod.rs` + `module/sub.rs`
- Types and utils should be in dedicated files/modules

### Adding a New Endpoint
1. Define request/response DTOs in `crates/0d-api/src/dto/`
2. Add handler in `crates/0d-api/src/handlers/`
3. Register route in `crates/0d-api/src/router.rs`
4. Add OpenAPI docs via `utoipa` derive macros
5. If it needs vault data, implement via `VaultMasterClient` trait in `crates/0d-master/`

### Adding a New Migration
1. `diesel migration generate <name>`
2. Edit `up.sql` and `down.sql`
3. Run `diesel migration run` — `schema.rs` auto-updates
4. Add/update Diesel models in `crates/0d-db/src/models/`

---

## Troubleshooting

### Common Issues

| Issue | Debug |
|-------|-------|
| Pod crash loop | `kubectl logs deploy/mainnet-0d-master-api-service -n mainnet --previous` |
| DB connection refused | Check CNPG cluster health: `kubectl get cluster zd-postgres-cluster -n mainnet` |
| Vault API errors | Check logs for `Liquidity endpoint not available` — some vault backends don't implement all endpoints |
| Indexer lag | Check `indexer_state` table for `last_block` vs current Starknet block |
| Rate limited | Production: 1 req/s with burst 100, whitelist: `app.0d.finance`, `earn.starknet.io` |

### Useful kubectl Commands

```bash
# Check deployment status
kubectl get deploy mainnet-0d-master-api-service -n mainnet

# Describe pod for events/errors
kubectl describe pod -l app.kubernetes.io/instance=mainnet-0d-master-api-service -n mainnet

# Check DB cluster health
kubectl get cluster zd-postgres-cluster -n mainnet

# Check ingress
kubectl get ingress 0d-master-api -n mainnet

# Restart deployment
kubectl rollout restart deploy/mainnet-0d-master-api-service -n mainnet
```
