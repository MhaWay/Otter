#!/usr/bin/env python3
"""
Otter Global Peer Registry

Mantieni una lista locale dei peer Otter noti e verifica la connettività.

USAGE:
  python3 peer_registry.py list          # Mostra i peer registrati
  python3 peer_registry.py add <id>      # Aggiungi un peer
  python3 peer_registry.py remove <id>   # Rimuovi un peer  
  python3 peer_registry.py check <id>    # Verifica connettività
"""

import json
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional

REGISTRY_FILE = Path.home() / ".otter" / "peer_registry.json"
REGISTRY_FILE.parent.mkdir(exist_ok=True)

class PeerRegistry:
    def __init__(self):
        self.peers: Dict[str, Dict] = self.load()
    
    def load(self) -> Dict:
        """Carica il registro dei peer"""
        if REGISTRY_FILE.exists():
            try:
                with open(REGISTRY_FILE) as f:
                    return json.load(f)
            except:
                return {}
        return {}
    
    def save(self):
        """Salva il registro"""
        with open(REGISTRY_FILE, "w") as f:
            json.dump(self.peers, f, indent=2)
    
    def add(self, peer_id: str, name: str = "", location: str = "") -> None:
        """Aggiungi un peer al registro"""
        self.peers[peer_id] = {
            "peer_id": peer_id,
            "name": name or f"Peer-{peer_id[:8]}",
            "location": location or "Unknown",
            "added_at": datetime.now().isoformat(),
            "last_seen": None,
            "connected": False
        }
        self.save()
        print(f"✓ Added peer: {self.peers[peer_id]['name']} ({peer_id[:16]}...)")
    
    def remove(self, peer_id: str) -> bool:
        """Rimuovi un peer"""
        if peer_id in self.peers:
            del self.peers[peer_id]
            self.save()
            print(f"✓ Removed peer: {peer_id}")
            return True
        print(f"❌ Peer not found: {peer_id}")
        return False
    
    def list(self) -> None:
        """Elenco tutti i peer registrati"""
        if not self.peers:
            print("📭 No peers registered")
            return
        
        print("\n" + "="*70)
        print("📋 REGISTERED OTTER PEERS")
        print("="*70)
        
        for peer_id, info in self.peers.items():
            status = "🟢 Online" if info.get("connected") else "⚪ Offline"
            print(f"\n{status} {info['name']}")
            print(f"   ID:       {peer_id}")
            print(f"   Location: {info['location']}")
            print(f"   Added:    {info['added_at']}")
            if info.get("last_seen"):
                print(f"   Seen:     {info['last_seen']}")
        
        print("\n" + "="*70)
    
    def mark_seen(self, peer_id: str) -> None:
        """Marca un peer come visto"""
        if peer_id in self.peers:
            self.peers[peer_id]["last_seen"] = datetime.now().isoformat()
            self.peers[peer_id]["connected"] = True
            self.save()
    
    def get_all_ids(self) -> List[str]:
        """Ritorna lista di tutti i PeerID"""
        return list(self.peers.keys())

def main():
    if len(sys.argv) < 2:
        print("Usage: peer_registry.py [list|add|remove|check]")
        return
    
    registry = PeerRegistry()
    command = sys.argv[1]
    
    if command == "list":
        registry.list()
    
    elif command == "add" and len(sys.argv) >= 3:
        peer_id = sys.argv[2]
        name = sys.argv[3] if len(sys.argv) > 3 else ""
        location = sys.argv[4] if len(sys.argv) > 4 else ""
        registry.add(peer_id, name, location)
    
    elif command == "remove" and len(sys.argv) >= 3:
        registry.remove(sys.argv[2])
    
    elif command == "check":
        print("\n✅ Registered peers (ready to connect):")
        for peer_id in registry.get_all_ids():
            peer = registry.peers[peer_id]
            print(f"   • {peer['name']}: {peer_id}")
        print(f"\nTotal: {len(registry.peers)} peers")
    
    else:
        print("Unknown command")

if __name__ == "__main__":
    main()
