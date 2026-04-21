#!/usr/bin/env bash
# ============================================================
# BOXMEOUT — Contract Deploy Script
# Deploys MarketFactory, Market wasm, and Treasury to Stellar.
#
# Usage: ./deploy.sh [testnet|mainnet]
#
# Contributors: implement the sections marked TODO.
# ============================================================

set -euo pipefail

NETWORK=${1:-testnet}
OUTPUT_FILE=".contract-addresses.env"

echo "Deploying to: $NETWORK"
echo "========================================"

# TODO: 1. Build all contracts
#   cargo build --release --target wasm32-unknown-unknown
#   stellar-cli contract optimize for each wasm

# TODO: 2. Deploy shared wasm (if needed as a library)

# TODO: 3. Deploy MarketFactory wasm
#   stellar contract deploy --wasm target/wasm32.../market_factory.wasm \
#     --source $ADMIN_SECRET_KEY --network $NETWORK
#   Save returned address as MARKET_FACTORY_ADDRESS

# TODO: 4. Deploy Market wasm (just upload the wasm hash; factory deploys instances)
#   stellar contract deploy --wasm target/wasm32.../market.wasm ...
#   Save returned wasm hash as MARKET_WASM_HASH

# TODO: 5. Deploy Treasury wasm
#   Save returned address as TREASURY_ADDRESS

# TODO: 6. Call MarketFactory::initialize(admin, default_fee_bps, oracles)

# TODO: 7. Call Treasury::initialize(admin, withdrawal_limit)

# TODO: 8. Write all addresses to $OUTPUT_FILE
#   echo "MARKET_FACTORY_ADDRESS=$MARKET_FACTORY_ADDRESS" >> $OUTPUT_FILE
#   echo "TREASURY_ADDRESS=$TREASURY_ADDRESS" >> $OUTPUT_FILE
#   echo "MARKET_WASM_HASH=$MARKET_WASM_HASH" >> $OUTPUT_FILE

echo "Deployment complete. Addresses saved to $OUTPUT_FILE"
