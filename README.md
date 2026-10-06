# _StrataBTC_

A personalized Bitcoin mining intelligence platform built in Rust. _StrataBTC_ combines
real-time Bitcoin network data, on-chain analysis, mining pool intelligence, and miner
performance tracking into a single interactive terminal UI.

It is designed for both aspiring miners researching the Bitcoin mining ecosystem and
active miners monitoring their operations — providing critical information without
requiring hardware connections or account registration.

---

## Vision

Most Bitcoin explorers answer the question: _what happened on the blockchain?_

_StrataBTC_ answers a different question: _what does it mean for Bitcoin mining?_

The platform focuses on two core areas:

**Blockchain Intelligence**
Explore blocks, transactions, addresses, UTXOs, coinbase transactions, mining activity,
and on-chain data — with context around what each means for the mining ecosystem.

**Mining Intelligence**
Understand the financial and operational reality of Bitcoin mining. Track network
hashrate, difficulty, pool performance, miner payout addresses, block production rates,
and estimated revenue. Detect performance gaps and quantify their economic impact.

The intended user lifecycle:

```
Interested in Bitcoin mining
        ↓
Explore mining information on _StrataBTC_
        ↓
Understand network conditions and pool activity
        ↓
Research hardware and pool economics
        ↓
Acquire hardware and join a pool
        ↓
Monitor mining performance against expectations
```

---

## Architecture

```
Bitcoin Network
      │
      ▼
Bitcoin Core / Chainstack RPC
      │  JSON-RPC over HTTPS
      ▼
┌─────────────────────────────┐
│   Rust Indexer              │
│                             │
│   - Fetches raw blocks      │
│   - Decodes with            │
│     rust-bitcoin            │
│   - Identifies mining pools │
│     from coinbase scripts   │
│   - Stores to PostgreSQL    │
│   - 5 concurrent fetches    │
│   - Retry on failure        │
└──────────────┬──────────────┘
               │
               ▼
        PostgreSQL
        (indexed data)
               │
               ▼
┌─────────────────────────────┐
│   Ratatui TUI               │
│                             │
│   - Split layout            │
│   - Block cards             │
│   - Network Stats panel     │
│   - Pool Intelligence panel │
│   - Miner Performance panel │
│   - Search                  │
│   - Detail screens          │
└─────────────────────────────┘
```

### Stack

| Layer         | Technology                                      |
|---------------|-------------------------------------------------|
| Bitcoin data  | Bitcoin Core (regtest) / Chainstack (mainnet)   |
| RPC client    | Custom `reqwest` async HTTP client              |
| Block decoder | `rust-bitcoin`                                  |
| Async runtime | Tokio                                           |
| Database      | PostgreSQL + SQLx                               |
| Terminal UI   | Ratatui + Crossterm                             |

### Database schema

| Table                  | Purpose                                      |
|------------------------|----------------------------------------------|
| `blocks`               | Block headers + pool name from coinbase      |
| `transactions`         | All transactions per block                   |
| `transaction_inputs`   | Inputs with previous outpoint references     |
| `transaction_outputs`  | Outputs with address and script type         |
| `indexer_state`        | Resume height for restart recovery           |

### Pool identification

When a block is indexed, the coinbase script is checked against 28 known pool
signatures (Foundry USA, AntPool, F2Pool, ViaBTC, Braiins, Luxor, MARA Pool, etc.).
The identified pool name is stored in `blocks.pool_name` and used to power the
Pool Intelligence screen.

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- PostgreSQL
- For regtest: [Polar](https://lightningpolar.com/)
- For mainnet: [Chainstack](https://chainstack.com/) Bitcoin node account

---

## Setup

### 1. Clone the project

```bash
git clone <your-repo-url>
cd block_explorer
```

### 2. Configure environment

```bash
cp .env.example .env
```

Edit `.env` with your credentials (see Network section below).

### 3. PostgreSQL

**Option A — Local:**
```bash
sudo systemctl start postgresql
sudo -u postgres createdb block_explorer
sudo -u postgres psql -c "ALTER USER postgres PASSWORD 'password';"
```

**Option B — Docker:**
```bash
docker compose up -d
```

### 4. Run migrations

```bash
sqlx migrate run --source migrations
```

### 5. Run

```bash
cargo run
```

---

## Networks

### Regtest (development)

Requires Polar running with a regtest network.

```env
RPC_URL=http://127.0.0.1:18443
RPC_USER=polaruser
RPC_PASSWORD=polarpass
BITCOIN_NETWORK=regtest
ZMQ_BLOCK_URL=tcp://127.0.0.1:28334
DATABASE_URL=postgres://postgres:password@localhost:5432/block_explorer_regtest
```

### Mainnet (via Chainstack)

Sign up at [chainstack.com](https://chainstack.com), create a Bitcoin mainnet node,
and copy the credentials to `.env`:

```env
RPC_URL=https://bitcoin-mainnet.core.chainstack.com
RPC_USER=your-chainstack-user
RPC_PASSWORD=your-chainstack-password
BITCOIN_NETWORK=mainnet
ZMQ_BLOCK_URL=tcp://127.0.0.1:28332
DATABASE_URL=postgres://postgres:password@localhost:5432/block_explorer
```

Use separate databases for each network to avoid data conflicts.

### Starting height

Mainnet has 900,000+ blocks. To start from a specific height:

```bash
sudo -u postgres psql -d block_explorer -c \
  "UPDATE indexer_state SET last_indexed_height = 969000;"
```

---

## TUI Controls

| Key / Action         | Effect                                    |
|----------------------|-------------------------------------------|
| Click search box     | Activate search                           |
| `/`                  | Activate search                           |
| `Enter`              | Submit search                             |
| `Esc`                | Cancel search / close detail screen       |
| `↑` / `↓`           | Scroll                                    |
| Mouse scroll         | Scroll                                    |
| Click right panel    | Open detail screen for that section       |
| `c`                  | Copy a field from search result           |
| `q` / `Ctrl+C`       | Quit                                      |

### Search accepts

| Input              | Resolves to          |
|--------------------|----------------------|
| `969000`           | Block at height      |
| `00000000000...`   | Block by hash        |
| `4a5e1e4baab8...`  | Transaction by TXID  |
| `bc1q...`          | Address              |

---

## Screens

### Home (split layout)

- **Left panel** — latest blocks with slide-in animations (newest first)
- **Right panel** — three clickable sections:
  - **Network Stats** — hashrate, difficulty, next adjustment, block subsidy
  - **Top Pools** — pool breakdown with bar charts
  - **Miner Performance** — top coinbase addresses with gap detection

### Network Stats detail

Full network overview: chain info, hashrate, difficulty, next adjustment estimate,
block subsidy, mempool size.

### Pool Intelligence detail

All identified mining pools with block count, share percentage, estimated hashrate,
and average block interval. Below each pool — its participating miner addresses
with their contribution percentage and estimated revenue.

### Miner Performance detail

Top 20 coinbase addresses ranked by blocks mined. Each card shows:
- Estimated hashrate
- Total revenue
- Average block interval
- Gap detection — if a miner has gone silent, the gap duration and estimated
  BTC lost are calculated and flagged with `⚠ GAP DETECTED`

---

## Project structure

```
block_explorer/
├── Cargo.toml
├── docker-compose.yml
├── .env.example
├── migrations/
│   ├── 001_indexer_state.sql
│   ├── 002_blocks.sql
│   ├── 003_transactions.sql
│   ├── 004_transaction_inputs.sql
│   ├── 005_transaction_outputs.sql
│   ├── 006_indexes.sql
│   └── 007_pool_name.sql
└── src/
    ├── main.rs
    ├── config.rs
    ├── errors.rs
    ├── rpc/
    │   ├── client.rs         # Async JSON-RPC client (Chainstack compatible)
    │   └── types.rs
    ├── indexer/
    │   ├── runner.rs         # Block sync with concurrency + retry
    │   ├── zmq.rs            # New block poller (every 5s)
    │   ├── mempool.rs        # Mempool poller (regtest only)
    │   ├── network.rs        # Network stats poller (every 30s)
    │   ├── pools.rs          # Pool identification from coinbase scripts
    │   └── display.rs
    ├── db/
    │   ├── blocks.rs         # Block/tx/input/output insertion
    │   ├── outputs.rs        # Mark outputs spent
    │   └── state.rs          # Indexer resume state
    └── tui/
        ├── app.rs            # Application state + animations
        ├── ui.rs             # Rendering (split layout + detail screens)
        ├── events.rs         # Keyboard/mouse input
        ├── queries.rs        # Database queries
        └── mod.rs            # TUI loop + shared state
```
