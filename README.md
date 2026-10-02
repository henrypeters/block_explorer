# _StrataBTC_

A Bitcoin block explorer built in Rust. Indexes blocks, transactions, inputs,
outputs and addresses from a Bitcoin Core node into PostgreSQL, then exposes
them through an interactive Ratatui terminal UI.

---

## Stack

| Layer        | Technology                               |
|--------------|------------------------------------------|
| Bitcoin node | Bitcoin Core (via Polar for dev)         |
| Indexer      | Rust — `bitcoincore-rpc`, `rust-bitcoin` |
| Database     | PostgreSQL                               |
| DB library   | SQLx                                     |
| UI           | Ratatui + Crossterm                      |
| Runtime      | Tokio                                    |

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Polar](https://lightningpolar.com/) — local Bitcoin Core regtest node
- PostgreSQL — either installed locally **or** via Docker

---

## Setup

### 1. Clone and enter the project

```bash
git clone <your-repo-url>
cd block_explorer
```

### 2. Configure environment

```bash
cp .env.example .env
```

Edit `.env` with your Bitcoin Core RPC credentials from Polar's connect tab.

---

## Running PostgreSQL

Pick one option.

---

### Option A — Local PostgreSQL (no Docker)

```bash
# Start PostgreSQL
sudo systemctl start postgresql

# Create the database
sudo -u postgres createdb block_explorer

# Set password (must match DATABASE_URL in .env)
sudo -u postgres psql -c "ALTER USER postgres PASSWORD 'password';"
```

`.env`:
```env
DATABASE_URL=postgres://postgres:password@localhost:5432/block_explorer
```

---

### Option B — Docker PostgreSQL

```bash
# Install Docker (Ubuntu)
sudo apt install docker.io docker-compose-plugin
sudo systemctl start docker
sudo usermod -aG docker $USER
# Log out and back in after this
```

```bash
# Start the database container
docker compose up -d
```

`.env`:
```env
DATABASE_URL=postgres://postgres:password@localhost:5432/block_explorer
```

The connection string is identical to Option A — Docker maps container
port 5432 to localhost:5432. No code changes needed between modes.

```bash
docker compose down    # stop (data is preserved in postgres_data volume)
```

---

## Run migrations

Run once after creating the database (works for both options):

```bash
sqlx migrate run --source migrations
```

---

## Start Bitcoin Core

Open Polar and start your regtest network. Your `.env` should have:

```env
RPC_URL=http://127.0.0.1:18443
RPC_USER=polaruser
RPC_PASSWORD=polarpass
BITCOIN_NETWORK=regtest
ZMQ_BLOCK_URL=tcp://127.0.0.1:28334
```

---

## Run the explorer

```bash
cargo run
```

On startup the explorer will:

1. Connect to Bitcoin Core and PostgreSQL
2. Index any blocks not yet in the database
3. Launch the TUI — blocks animate in from alternating directions
4. Poll for new blocks every 5 seconds in the background

---

## TUI Controls

| Key / Action     | Effect                           |
|------------------|----------------------------------|
| Click search box | Activate search                  |
| `/`              | Activate search                  |
| `Enter`          | Submit search                    |
| `Esc`            | Cancel search / clear results    |
| `↑` / `↓`       | Scroll                           |
| Mouse scroll     | Scroll                           |
| `c`              | Copy a field from current result |
| `q` / `Ctrl+C`   | Quit                             |

### Search accepts

| Input             | Resolves to         |
|-------------------|---------------------|
| `5`               | Block at height 5   |
| `0f9188f13cb7...` | Block by hash       |
| `4a5e1e4baab8...` | Transaction by TXID |
| `bcrt1q...`       | Address             |

---

## Project Structure

```
block_explorer/
├── Cargo.toml
├── docker-compose.yml          # PostgreSQL via Docker
├── .env                        # Local config (git-ignored)
├── .env.example                # Template
├── migrations/
│   ├── 001_indexer_state.sql
│   ├── 002_blocks.sql
│   ├── 003_transactions.sql
│   ├── 004_transaction_inputs.sql
│   └── 005_transaction_outputs.sql
└── src/
    ├── main.rs
    ├── config.rs               # Reads from .env
    ├── errors.rs
    ├── rpc/
    │   ├── client.rs           # Bitcoin Core RPC client
    │   └── types.rs
    ├── indexer/
    │   ├── runner.rs           # Initial sync
    │   ├── zmq.rs              # Polls for new blocks
    │   └── display.rs          # Terminal block display
    ├── db/
    │   ├── blocks.rs           # Insert blocks/txs/inputs/outputs
    │   ├── outputs.rs          # Mark outputs spent
    │   └── state.rs            # Indexer resume state
    └── tui/
        ├── app.rs              # Application state + animations
        ├── ui.rs               # Rendering
        ├── events.rs           # Keyboard/mouse input
        └── queries.rs          # Database queries for search
```

---

## Development workflow

```bash
# Start postgres
docker compose up -d          # Docker
# OR
sudo systemctl start postgresql  # local

# Start Polar and your regtest network

# Run the explorer
cargo run
```

Mine a block to see it appear live:

```bash
bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 \
  generatetoaddress 1 $(bitcoin-cli -regtest -rpcuser=polaruser \
  -rpcpassword=polarpass -rpcport=18443 getnewaddress)
```



```
A personalized Bitcoin mining and blockchain intelligence explorer that combines real-time Bitcoin network data, mining data, miner-specific performance data, and transaction-analysis logic. The explorer provides standard blockchain search functionality while focusing primarily on Bitcoin mining and the broader mining ecosystem. It is designed for both existing miners and aspiring miners who may not yet own mining hardware, giving users access to a broad range of information and intelligence about the Bitcoin mining world.

The explorer will focus primarily on two core areas: Economics Intelligence and Blockchain Intelligence.

Economics Intelligence will help users understand the financial and operational side of Bitcoin mining by providing information such as mining profitability, electricity costs, hardware economics, network difficulty, network hashrate, pool fees and reward structures, expected BTC production, mining revenue, operating costs, and different mining scenarios. Users will be able to model potential mining operations based on factors such as available capital, electricity costs, hashrate, hardware, and pool participation, while existing miners can connect their actual mining data to compare real-world performance against expected outcomes.

Blockchain Intelligence will provide users with a deeper understanding of the Bitcoin network by allowing them to explore blocks, transactions, addresses, UTXOs, the mempool, mining activity, coinbase transactions, fees, network difficulty, network hashrate, and other on-chain activity. The system will analyze blockchain data to provide context around what is happening in the network and how those events relate to Bitcoin mining.

The explorer will track current mainnet difficulty and network hashrate and use this information together with a user's hashrate, if the user already has a machine, to calculate personalized expected block production and the probability of finding a block over different time periods. For existing miners, it can also analyze miner-specific performance and help quantify the financial consequences of problems such as reduced hashrate, downtime, overheating, hardware degradation, or other operational issues by estimating their effect on expected Bitcoin production and mining revenue.

The platform will also provide mining ecosystem intelligence, allowing users to study and compare mining pools, their activity, hashrate, reward structures, fees, payouts, and other relevant characteristics. This information will be useful both to existing miners evaluating their current mining operation and to aspiring miners researching how to enter the Bitcoin mining ecosystem.

Users will create accounts not only to connect existing mining hardware or mining pools, but also to build a personalized mining profile before they own any hardware. An aspiring miner can use the platform to learn about the mining ecosystem, model hypothetical mining operations, investigate hardware and pool economics, and understand how changes in Bitcoin's network conditions can affect potential mining outcomes. As the user progresses from researching mining to operating actual hardware, the same account can become connected to their mining machines, pool accounts, and operational data.

The overall goal is to create more than a conventional blockchain explorer: a personalized Bitcoin mining intelligence platform that helps users understand what is happening in the Bitcoin network, what it means economically, and what those conditions mean for their own potential or existing mining operation.

The intended user lifecycle is:

Interested in Bitcoin mining → Create account → Explore mining information → Model potential mining operations → Research hardware and pools → Acquire hardware → Connect mining equipment → Monitor actual performance → Compare actual performance against expected outcomes → Analyze and improve the mining operation.
```