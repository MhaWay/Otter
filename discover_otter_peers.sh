#!/bin/bash
#
# discover_otter_peers.sh
# Scopri tutti i peer Otter attivi sulla rete locale e Internet
#
# USAGE:
#   ./discover_otter_peers.sh

set -e

echo "🚀 OTTER PEER DISCOVERY TOOL"
echo "=========================================="
echo ""

# Verifica se il test è compilato
if [ ! -f "./target/release/examples/bootstrap_test" ]; then
    echo "📦 Building test binary..."
    cargo build --example bootstrap_test --release > /dev/null
fi

echo "🌐 Connecting to libp2p DHT network..."
echo "   (This may take 10-30 seconds)"
echo ""

# Esegui il bootstrap test e estrai i PeerID
cargo run --example bootstrap_test --release 2>&1 | tee /tmp/peer_discovery.log | grep -E "Connected to:|Local PeerId:|Identity received" || true

echo ""
echo "📊 Summary:"
echo "=========================================="

# Conta i peer scoperti
DISCOVERED=$(grep -c "Connected to:" /tmp/peer_discovery.log || echo 0)
echo "✓ Connected peers: $DISCOVERED"

# Mostra i PeerID
echo ""
echo "🆔 Peer IDs found:"
grep -oP "Connected to: \K[^ ]+" /tmp/peer_discovery.log | sort -u || echo "   (none)"

echo ""
echo "💾 Full log saved to: /tmp/peer_discovery.log"
echo "   View with: cat /tmp/peer_discovery.log"
