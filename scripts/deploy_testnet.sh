#!/usr/bin/env bash
# ==============================================================================
# BOXMEOUT — Deploy all contracts to Stellar Testnet
# ==============================================================================
#
# Deploys Market (WASM), MarketFactory, and Treasury to Stellar Testnet in a
# single command. Wires all contract addresses together so they can communicate.
#
# Requirements:
#   - Rust + wasm32-unknown-unknown target
#   - Stellar CLI (soroban) v21+
#   - Funded Testnet keypair
#   - jq (for JSON output)
#
# Usage:
#   ADMIN_SECRET=SABC... ./deploy_testnet.sh
#
# Output:
#   - Contract IDs printed to stdout
#   - .env.testnet (shell env file)
#   - ../../config.json (JSON consumable by backend/frontend)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENV_FILE="$SCRIPT_DIR/.env.testnet"
JSON_FILE="$PROJECT_ROOT/config.json"

# ─── VALIDATE ENVIRONMENT ─────────────────────────────────────────────────────

if [[ -z "${ADMIN_SECRET:-}" ]]; then
  echo "ERROR: ADMIN_SECRET is not set. Export your Testnet secret key first."
  echo "  export ADMIN_SECRET=SABC..."
  exit 1
fi

for cmd in cargo soroban jq; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "ERROR: '$cmd' is not installed or not in PATH."
    exit 1
  fi
done

NETWORK="testnet"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
NETWORK_URL="https://soroban-testnet.stellar.org"
ADMIN_PUBKEY=$(soroban keys address "$ADMIN_SECRET" 2>/dev/null || echo "")

if [[ -z "$ADMIN_PUBKEY" ]]; then
  echo "ERROR: Could not derive public key from ADMIN_SECRET."
  exit 1
fi

echo "==> Admin public key: $ADMIN_PUBKEY"

# ─── BUILD ────────────────────────────────────────────────────────────────────

echo ""
echo "==> Building all contracts (release, wasm32-unknown-unknown)..."
cd "$CONTRACTS_DIR"
cargo build --release --target wasm32-unknown-unknown 2>&1

WASM_DIR="$CONTRACTS_DIR/target/wasm32-unknown-unknown/release"

# ─── OPTIMIZE ─────────────────────────────────────────────────────────────────

echo ""
echo "==> Optimizing WASM binaries..."
for contract in market treasury market_factory; do
  WASM_IN="$WASM_DIR/${contract}.wasm"
  WASM_OUT="$WASM_DIR/${contract}.optimized.wasm"

  if [[ ! -f "$WASM_IN" ]]; then
    echo "ERROR: $WASM_IN not found after build."
    exit 1
  fi

  soroban contract optimize \
    --wasm       "$WASM_IN" \
    --wasm-out   "$WASM_OUT"
  echo "    Optimized: $WASM_OUT"
done

# ─── INSTALL MARKET WASM ──────────────────────────────────────────────────────
# Upload the Market contract WASM to the network so MarketFactory can deploy
# new instances from it.

echo ""
echo "==> Installing Market contract WASM on-chain..."
MARKET_WASM_HASH=$(soroban contract install \
  --wasm       "$WASM_DIR/market.optimized.wasm" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL")

echo "    Market WASM hash: $MARKET_WASM_HASH"

# ─── DEPLOY & INITIALIZE MARKET FACTORY ───────────────────────────────────────

echo ""
echo "==> Deploying MarketFactory..."
FACTORY_ID=$(soroban contract deploy \
  --wasm       "$WASM_DIR/market_factory.optimized.wasm" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL")

echo "    MarketFactory contract ID: $FACTORY_ID"

echo ""
echo "==> Initializing MarketFactory..."
soroban contract invoke \
  --id         "$FACTORY_ID" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL" \
  -- initialize \
  --admin         "$ADMIN_PUBKEY" \
  --fee-collector "PLACEHOLDER" \
  --default-fee-bp 200 \
  --min-bet       1000000 \
  --max-bet       100000000000

# ─── DEPLOY & INITIALIZE TREASURY ─────────────────────────────────────────────

echo ""
echo "==> Deploying Treasury..."
TREASURY_ID=$(soroban contract deploy \
  --wasm       "$WASM_DIR/treasury.optimized.wasm" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL")

echo "    Treasury contract ID: $TREASURY_ID"

echo ""
echo "==> Initializing Treasury with Factory address..."
soroban contract invoke \
  --id         "$TREASURY_ID" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL" \
  -- initialize \
  --admin      "$ADMIN_PUBKEY" \
  --factory    "$FACTORY_ID"

# ─── WIRE FACTORY WITH MARKET WASM ────────────────────────────────────────────

echo ""
echo "==> Setting Market WASM hash on Factory..."
soroban contract invoke \
  --id         "$FACTORY_ID" \
  --source     "$ADMIN_SECRET" \
  --network    "$NETWORK" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --rpc-url    "$NETWORK_URL" \
  -- update_market_wasm \
  --admin      "$ADMIN_PUBKEY" \
  --new-wasm-hash "$MARKET_WASM_HASH"

# ─── SAVE OUTPUT ──────────────────────────────────────────────────────────────

echo ""
echo "==> Saving contract IDs..."

# Shell env file for scripts
cat > "$ENV_FILE" <<ENVFILE
# BOXMEOUT Stellar Testnet — generated by deploy_testnet.sh
NETWORK=testnet
RPC_URL=${NETWORK_URL}
ADMIN_PUBKEY=${ADMIN_PUBKEY}
TREASURY_CONTRACT_ID=${TREASURY_ID}
FACTORY_CONTRACT_ID=${FACTORY_ID}
MARKET_WASM_HASH=${MARKET_WASM_HASH}
ENVFILE

# JSON file consumable by backend/frontend
cat > "$JSON_FILE" <<JSONFILE
{
  "network": "testnet",
  "rpcUrl": "${NETWORK_URL}",
  "networkPassphrase": "${NETWORK_PASSPHRASE}",
  "adminPublicKey": "${ADMIN_PUBKEY}",
  "contracts": {
    "treasury": "${TREASURY_ID}",
    "marketFactory": "${FACTORY_ID}",
    "marketWasmHash": "${MARKET_WASM_HASH}"
  },
  "protocolConfig": {
    "defaultFeeBp": 200,
    "minBet": 1000000,
    "maxBet": 100000000000
  }
}
JSONFILE

echo ""
echo "============================================================"
echo "  DEPLOYMENT COMPLETE"
echo "============================================================"
echo "  Treasury:         $TREASURY_ID"
echo "  MarketFactory:    $FACTORY_ID"
echo "  Market WASM hash: $MARKET_WASM_HASH"
echo "============================================================"
echo ""
echo "  Shell env file: $ENV_FILE"
echo "  JSON config:    $JSON_FILE"
echo ""
echo "Backend/frontend can import config.json for contract addresses."
