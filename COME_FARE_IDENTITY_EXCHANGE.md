# Come Fare Identity Exchange in Otter

**Domanda originale**: "come dovrei fare l'identity excange"

## Risposta Breve

**Non devi fare nulla!** L'identity exchange è ora completamente automatico.

## Come Funziona

### 1. Avvia Otter

```bash
./otter
```

### 2. Connettiti ai Peer

La connessione avviene automaticamente tramite:
- **mDNS**: Trova peer sulla rete locale
- **Kademlia DHT**: Scopre peer in Internet

### 3. Identity Exchange Automatico

Quando due peer si connettono:
```
✓ Connected: <peer_id>
  ✓ Identity sent         ← AUTOMATICO!
✓ Identity verified       ← AUTOMATICO!
```

### 4. Inizia a Chattare

Ora puoi inviare messaggi criptati:
```
/send
Seleziona il peer
Messaggio: Ciao!
```

## Cosa Succede Dietro le Quinte

```
┌───────────────┐                     ┌───────────────┐
│  Tu (Peer A)  │                     │ Amico (Peer B)│
└───────┬───────┘                     └───────┬───────┘
        │                                     │
        │  1. Scoperta peer                   │
        │◄────────────────────────────────────┤
        │                                     │
        │  2. Connessione P2P                 │
        │◄────────────────────────────────────┤
        │                                     │
        │  3. Invio automatico identità       │
        ├────────►│                            │
        │         │  Identity(A)              │
        │         └───────────────────────────►│
        │                                     │
        │  4. Ricezione identità amico        │
        │◄────────────────────────────────────┤
        │  Identity(B)                        │
        │                                     │
        │  5. Scambio chiavi (X25519)         │
        │◄───────────────────────────────────►│
        │                                     │
        │  ✓ Pronto per chat criptata        │
        │                                     │
        │  6. Messaggio criptato              │
        ├────────────────────────────────────►│
        │  ChaCha20-Poly1305                  │
        │                                     │
└───────┴─────────────────────────────────────┴───────┘
```

## Dettagli Tecnici

### Cosa Viene Scambiato

1. **Peer ID**: Identificatore unico derivato dalla chiave pubblica
2. **Chiave Ed25519**: Per firmare e verificare messaggi
3. **Chiave X25519**: Per criptare messaggi

### Come Funziona la Crittografia

1. **Scambio chiavi**: Protocollo X25519 (Diffie-Hellman)
2. **Cifratura**: ChaCha20-Poly1305 (AEAD)
3. **Firma**: Ed25519 (curve ellittiche)
4. **Hashing**: BLAKE3 per Peer ID

### Sicurezza

✅ **Cifratura end-to-end**: Solo tu e il destinatario potete leggere
✅ **Autenticazione**: Le chiavi pubbliche sono verificate
✅ **Integrità**: I messaggi non possono essere modificati

⚠️ **Cosa verificare**:
- Confronta il "fingerprint" con il tuo amico (telefono/di persona)
- Esempio: `2945f80a` (primi 8 byte della chiave pubblica)

## Codice (Per Sviluppatori)

### Invio Identità

```rust
// Creazione messaggio identità
let identity_msg = Message::identity(handler.public_identity());
let data = identity_msg.to_bytes()?;

// Invio al peer
command_tx.send(NetworkCommand::SendMessage {
    to: peer_id,
    data,
}).await?;
```

### Ricezione Identità

```rust
match message {
    Message::Identity { public_identity, .. } => {
        // Registra il peer
        handler.register_peer(public_identity)?;
        println!("✓ Identità verificata");
    }
}
```

## Test

### Terminal 1 (Alice)
```bash
./otter --nickname Alice
```

### Terminal 2 (Bob)
```bash
./otter --nickname Bob --port 9001
```

### Risultato Atteso

**Entrambi i terminali mostrano**:
```
✓ Discovered peer: <peer_id>
✓ Connected: <peer_id>
  ✓ Identity sent
✓ Identity verified for peer: <peer_id>
```

### Prova Messaggi Criptati

**Alice**:
```
otter> /send
Seleziona: Bob
Messaggio: Ciao Bob!
✓ Messaggio criptato e inviato!
```

**Bob vede**:
```
🔐 Messaggio da Alice: Ciao Bob!
```

## Risoluzione Problemi

### "Peer not found" quando invio messaggio

**Problema**: Identity exchange non completato  
**Soluzione**: Aspetta il messaggio "✓ Identity verified" prima di inviare

### Peer si disconnette subito

**Problema**: Problemi di rete o firewall  
**Soluzione**: 
- Controlla firewall
- Usa `--port` per specificare porta
- Verifica connessione di rete

### Fingerprint diverso dal previsto

**Problema**: Peer ha rigenerato identità o attacco MITM  
**Soluzione**: Verifica fingerprint con il peer prima di continuare

## Domande Frequenti

**Q: Devo configurare qualcosa?**  
R: No, funziona automaticamente.

**Q: Posso disabilitare lo scambio automatico?**  
R: No, è necessario per la crittografia.

**Q: Come verifico l'identità di un peer?**  
R: Confronta i fingerprint tramite un canale sicuro (telefono, di persona).

**Q: Posso usare lo stesso Peer ID su dispositivi diversi?**  
R: No, ogni dispositivo ha un Peer ID unico. (Multi-device è pianificato)

**Q: Cosa succede se perdo la mia identità?**  
R: Avrai un nuovo Peer ID. Backup `~/.otter/identity.json`!

## Documentazione Aggiuntiva

Per maggiori dettagli:

- **IDENTITY_EXCHANGE.md**: Documentazione tecnica completa
- **IDENTITY_EXCHANGE_GUIDE.md**: Guida utente in inglese
- **QUICKSTART.md**: Guida rapida per iniziare
- **ARCHITECTURE.md**: Architettura del sistema

## Riepilogo

### In Pratica

1. **Avvia**: `./otter`
2. **Connetti**: Automatico
3. **Scambia identità**: Automatico
4. **Chatta**: `/send`

### Zero Configurazione

✅ Nessuna configurazione manuale  
✅ Nessun setup richiesto  
✅ Nessuna generazione manuale di chiavi  
✅ Funziona immediatamente  

### Sicuro Per Default

✅ Crittografia end-to-end automatica  
✅ Chiavi pubbliche verificate  
✅ Nessun server centrale  
✅ Privacy protetta  

---

**Conclusione**: L'identity exchange in Otter è completamente automatico e trasparente. Devi solo avviare il programma e connetterti - tutto il resto viene gestito automaticamente! 🦦

## Implementazione Tecnica

### File Modificati

1. **crates/otter-cli/src/main.rs**: Implementazione dello scambio automatico
2. **crates/otter-messaging/src/lib.rs**: Tipo messaggio Identity
3. **crates/otter-identity/src/lib.rs**: Gestione identità e chiavi

### Flusso del Codice

```rust
// 1. Evento di connessione
NetworkEvent::PeerConnected { peer_id } => {
    // 2. Crea messaggio identità
    let identity_msg = Message::identity(handler.public_identity());
    
    // 3. Serializza
    let data = identity_msg.to_bytes()?;
    
    // 4. Invia automaticamente
    command_tx.send(NetworkCommand::SendMessage {
        to: peer_id,
        data,
    }).await?;
}

// 5. Ricezione sul peer remoto
NetworkEvent::MessageReceived { from, data } => {
    if let Ok(Message::Identity { public_identity, .. }) = Message::from_bytes(&data) {
        // 6. Registra peer e crea sessione cripto
        handler.register_peer(public_identity)?;
    }
}
```

### Protocolli Utilizzati

- **libp2p**: Networking P2P
- **Ed25519**: Firma digitale
- **X25519**: Scambio chiavi
- **ChaCha20-Poly1305**: Cifratura AEAD
- **BLAKE3**: Hashing per Peer ID

**Tutto questo avviene automaticamente quando avvii Otter!** 🎉
