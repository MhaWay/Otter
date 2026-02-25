#!/usr/bin/env python3
"""
Otter Peer Discovery Tool

Discover all active Otter peers on the network (local and Internet).
Connects to libp2p DHT and searches for peers running Otter.

USAGE:
  python3 discover_peers.py [--timeout SECONDS] [--max-peers N]
"""

import subprocess
import json
import time
import re
import sys
import argparse
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional

class PeerDiscovery:
    def __init__(self, timeout: int = 60, max_peers: int = 100):
        self.timeout = timeout
        self.max_peers = max_peers
        self.peers: Dict[str, Dict] = {}
        self.start_time = datetime.now()
        
    def run_bootstrap_test(self) -> str:
        """Esegui il test di bootstrap e cattura l'output"""
        print("🌐 Avvio DHT discovery (timeout: {}s)...".format(self.timeout))
        print("")
        
        try:
            # Esegui il test con timeout
            result = subprocess.run(
                ["cargo", "run", "--example", "bootstrap_test", "--release"],
                capture_output=True,
                text=True,
                timeout=self.timeout + 10,
                cwd="/home/mhaway/Otter"
            )
            return result.stdout + result.stderr
        except subprocess.TimeoutExpired:
            return "TIMEOUT"
        except Exception as e:
            print(f"❌ Errore: {e}")
            return ""
    
    def parse_connections(self, output: str) -> List[Dict]:
        """Estrai i peer scoperti dall'output"""
        peers = []
        
        # Pattern: "✅ Connected to: <PeerID> (...)"
        pattern = r"✅ Connected to: ([^ ]+)"
        matches = re.findall(pattern, output)
        
        for peer_id in matches:
            if peer_id not in self.peers:
                self.peers[peer_id] = {
                    "peer_id": peer_id,
                    "discovered_at": datetime.now().isoformat(),
                    "type": "bootstrap"  # Sono peer bootstrap pubblici
                }
                peers.append(self.peers[peer_id])
        
        return peers
    
    def parse_dht_routing(self, output: str) -> List[Dict]:
        """Estrai i peer dalla DHT routing table"""
        peers = []
        
        # Pattern: "📡 DHT routing updated: <PeerID> added"
        pattern = r"📡 DHT routing updated: ([^ ]+) added"
        matches = re.findall(pattern, output)
        
        for peer_id in matches:
            if peer_id not in self.peers:
                self.peers[peer_id] = {
                    "peer_id": peer_id,
                    "discovered_at": datetime.now().isoformat(),
                    "type": "dht"
                }
                peers.append(self.peers[peer_id])
        
        return peers
    
    def print_results(self, output: str):
        """Stampa i risultati della scoperta"""
        connected = self.parse_connections(output)
        dht = self.parse_dht_routing(output)
        
        print("\n" + "="*60)
        print("📊 PEER DISCOVERY RESULTS")
        print("="*60)
        
        # Riepilogo
        total_unique = len(self.peers)
        print(f"\n✓ Total unique peers discovered: {total_unique}")
        
        if total_unique == 0:
            print("\n⚠️  No peers discovered on DHT")
            print("   Reasons:")
            print("   - Network connectivity issues")
            print("   - Firewall blocking p2p connections")
            print("   - No other Otter peers currently active")
            return
        
        # Peer pubblici (bootstrap)
        bootstrap_peers = [p for p in self.peers.values() if p["type"] == "bootstrap"]
        if bootstrap_peers:
            print(f"\n🌍 Bootstrap Peers (libp2p public network): {len(bootstrap_peers)}")
            for peer in bootstrap_peers[:5]:
                print(f"   • {peer['peer_id'][:16]}...")
            if len(bootstrap_peers) > 5:
                print(f"   ... and {len(bootstrap_peers) - 5} more")
        
        # Peer da DHT
        dht_peers = [p for p in self.peers.values() if p["type"] == "dht"]
        if dht_peers:
            print(f"\n📡 DHT Peers (potential Otter nodes): {len(dht_peers)}")
            for peer in dht_peers[:5]:
                print(f"   • {peer['peer_id'][:16]}...")
            if len(dht_peers) > 5:
                print(f"   ... and {len(dht_peers) - 5} more")
        
        # Salva en JSON per processamento
        print("\n💾 Saving peer list...")
        self.save_peers_json()
        
        print("="*60)
        
        # Analisi connettività
        print("\n🔍 NETWORK ANALYSIS")
        print("="*60)
        
        if "FIRST PEER CONNECTED in" in output:
            # Estrai il tempo alla prima connessione
            match = re.search(r"FIRST PEER CONNECTED in ([\d.]+)s", output)
            if match:
                time_to_first = float(match.group(1))
                status = "✅ Fast" if time_to_first < 5 else "⚠️  Slow"
                print(f"{status} - Time to first peer: {time_to_first:.2f}s")
        
        # Routing table size
        if "Routing table:" in output:
            match = re.search(r"Routing table: (\d+) peers", output)
            if match:
                rt_size = int(match.group(1))
                status = "✅ Healthy" if rt_size >= 10 else "⚠️  Small"
                print(f"{status} - DHT routing table size: {rt_size} peers")
        
        # Success rate
        if "Successful:" in output:
            match = re.search(r"Successful: (\d+) \(([\d.]+)%\)", output)
            if match:
                success_count = int(match.group(1))
                success_rate = float(match.group(2))
                print(f"Connection success rate: {success_rate:.1f}% ({success_count} successful)")
        
        print("="*60)
    
    def save_peers_json(self):
        """Salva la lista di peer in JSON"""
        output_file = Path("/tmp/otter_peers.json")
        data = {
            "discovered_at": self.start_time.isoformat(),
            "peers_count": len(self.peers),
            "peers": list(self.peers.values())
        }
        
        with open(output_file, "w") as f:
            json.dump(data, f, indent=2)
        
        print(f"   Written to: {output_file}")
    
    def discover(self):
        """Esegui la scoperta completa"""
        print("\n" + "="*60)
        print("🚀 OTTER PEER DISCOVERY")
        print("="*60)
        print(f"\nStart time: {self.start_time.strftime('%H:%M:%S')}")
        print(f"Timeout: {self.timeout}s")
        print("\n")
        
        # Esegui il bootstrap test
        output = self.run_bootstrap_test()
        
        if output == "TIMEOUT":
            print("❌ Discovery timed out")
            return False
        
        if not output:
            print("❌ No output from discovery")
            return False
        
        # Analizza i risultati
        self.print_results(output)
        
        return len(self.peers) > 0

def main():
    parser = argparse.ArgumentParser(
        description="Discover Otter peers on libp2p DHT network"
    )
    parser.add_argument(
        "--timeout", 
        type=int, 
        default=60,
        help="Discovery timeout in seconds (default: 60)"
    )
    parser.add_argument(
        "--max-peers",
        type=int,
        default=100,
        help="Maximum peers to track (default: 100)"
    )
    
    args = parser.parse_args()
    
    discovery = PeerDiscovery(timeout=args.timeout, max_peers=args.max_peers)
    success = discovery.discover()
    
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
