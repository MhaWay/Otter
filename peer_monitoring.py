#!/usr/bin/env python3
"""
Otter Peer Lookup and Connection Monitor

Per usare questo strumento:
1. Prendi i PeerID dalle tue app Otter (generalmente in formato 12D3Koow...)
2. Registrali con: python3 peer_monitoring.py register <PEER_ID> <YOUR_NAME>
3. Monitora le connessioni con: python3 peer_monitoring.py monitor

USAGE:
  python3 peer_monitoring.py register <peer_id> <name>
  python3 peer_monitoring.py monitor [peer_id_to_search]
  python3 peer_monitoring.py status
"""

import subprocess
import json
import time
import re
import sys
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Dict

CONFIG_DIR = Path.home() / ".otter"
CONFIG_FILE = CONFIG_DIR / "peer_monitoring.json"
CONFIG_DIR.mkdir(exist_ok=True)

class PeerMonitor:
    def __init__(self):
        self.config = self.load_config()
    
    def load_config(self) -> Dict:
        """Carica la configurazione"""
        if CONFIG_FILE.exists():
            with open(CONFIG_FILE) as f:
                return json.load(f)
        return {"peers": {}, "last_check": None}
    
    def save_config(self):
        """Salva la configurazione"""
        with open(CONFIG_FILE, "w") as f:
            json.dump(self.config, f, indent=2)
    
    def register_peer(self, peer_id: str, name: str):
        """Registra un peer noto"""
        self.config["peers"][peer_id] = {
            "name": name,
            "registered_at": datetime.now().isoformat(),
            "last_seen": None,
            "is_online": False
        }
        self.save_config()
        print(f"✅ Registrato: {name}")
        print(f"   PeerID: {peer_id}")
    
    def show_status(self):
        """Mostra lo status dei peer registrati"""
        if not self.config["peers"]:
            print("📭 Nessun peer registrato")
            print("\nPer aggiungere un peer:")
            print("  python3 peer_monitoring.py register <PEER_ID> <NOME>")
            return
        
        print("\n" + "="*70)
        print("📊 PEER MONITORING STATUS")
        print("="*70)
        
        for peer_id, info in self.config["peers"].items():
            status = "🟢 ONLINE" if info["is_online"] else "⚫ OFFLINE"
            print(f"\n{status} {info['name']}")
            print(f"   PeerID: {peer_id}")
            print(f"   Registered: {info['registered_at']}")
            if info.get("last_seen"):
                print(f"   Last seen: {info['last_seen']}")
        
        print("\n" + "="*70)
        
        online_count = sum(1 for p in self.config["peers"].values() if p["is_online"])
        total_count = len(self.config["peers"])
        
        print(f"\n📈 Summary: {online_count}/{total_count} peers online")
        if online_count == 0:
            print("\n⚠️  No peers currently online")
            print("   Assicurati che le app Otter siano in esecuzione")
    
    def search_and_monitor_peer(self, target_peer_id: Optional[str] = None):
        """Cerca e monitora un peer specifico o tutti"""
        print("\n" + "="*70)
        print("🔍 SEARCHING FOR OTTER PEERS ON DHT")
        print("="*70 + "\n")
        
        if not target_peer_id and not self.config["peers"]:
            print("❌ Nessun peer da cercare!")
            print("   Registra prima i PeerID con: python3 peer_monitoring.py register <PEER_ID> <NOME>")
            return
        
        targets = [target_peer_id] if target_peer_id else list(self.config["peers"].keys())
        
        print(f"🎯 Searching for {len(targets)} peer(s)...")
        print(f"   Timeout: 30 seconds\n")
        
        for peer_id in targets:
            peer_name = self.config["peers"].get(peer_id, {}).get("name", "Unknown")
            print(f"🔎 Searching: {peer_name} ({peer_id[:16]}...)")
            
            try:
                # Usa il test peer_search_test
                result = subprocess.run(
                    ["cargo", "run", "--example", "peer_search_test", "--release", "--", peer_id],
                    capture_output=True,
                    text=True,
                    timeout=35,
                    cwd="/home/mhaway/Otter"
                )
                
                output = result.stdout + result.stderr
                
                if "Found target" in output or "Connected peers observed:" in output:
                    print(f"   ✅ FOUND!")
                    if peer_id in self.config["peers"]:
                        self.config["peers"][peer_id]["is_online"] = True
                        self.config["peers"][peer_id]["last_seen"] = datetime.now().isoformat()
                else:
                    print(f"   ⏳ Searching (may not be currently discoverable)...")
                    if peer_id in self.config["peers"]:
                        self.config["peers"][peer_id]["is_online"] = False
                
                # Estrai numero di peer connessi
                match = re.search(r"Connected peers observed: (\d+)", output)
                if match:
                    connected_count = int(match.group(1))
                    print(f"   └─ Connected peers observed on network: {connected_count}")
            
            except subprocess.TimeoutExpired:
                print(f"   ⏱️  Timeout (peer may be offline or unreachable)")
                if peer_id in self.config["peers"]:
                    self.config["peers"][peer_id]["is_online"] = False
            
            except Exception as e:
                print(f"   ❌ Error: {e}")
            
            print()
        
        self.config["last_check"] = datetime.now().isoformat()
        self.save_config()
        self.show_status()

def main():
    monitor = PeerMonitor()
    
    if len(sys.argv) < 2:
        print("OTTER PEER DISCOVERY & MONITORING")
        print("\nUsage:")
        print("  python3 peer_monitoring.py register <PEER_ID> <NAME>")
        print("    Registra un peer da monitorare")
        print("    Esempio: python3 peer_monitoring.py register 12D3KooWXYZ... 'MyAppPC2'")
        print()
        print("  python3 peer_monitoring.py monitor [PEER_ID]")
        print("    Cerca il peer sulla rete DHT")
        print("    Se PEER_ID omesso, cerca tutti i peer registrati")
        print()
        print("  python3 peer_monitoring.py status")
        print("    Mostra lo stato dei peer")
        return
    
    command = sys.argv[1]
    
    if command == "register" and len(sys.argv) >= 4:
        peer_id = sys.argv[2]
        name = sys.argv[3]
        monitor.register_peer(peer_id, name)
    
    elif command == "monitor":
        target = sys.argv[2] if len(sys.argv) >= 3 else None
        monitor.search_and_monitor_peer(target)
    
    elif command == "status":
        monitor.show_status()
    
    else:
        print("Unknown command")

if __name__ == "__main__":
    main()
