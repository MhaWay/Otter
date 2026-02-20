# Informativa sulla Privacy - Otter App

**Data ultimo aggiornamento:** Febbraio 2026

## 1. Introduzione

Otter ("l'App") è un'applicazione di messaggistica peer-to-peer (P2P) che consente comunicazioni dirette tra utenti. Questa informativa sulla privacy spiega come raccogliamo, utilizziamo e proteggiamo i dati personali.

**Titolare del Trattamento:** [Inserire dati dell'azienda/sviluppatore]

## 2. Dati Raccolti

### 2.1 Dati da Google OAuth
Quando accedi tramite Google, raccogliamo:
- **Email** (per identificazione e contatti)
- **Nome** (per il profilo)
- **Foto profilo** (opzionale, per visualizzazione)

### 2.2 Dati di Contatti
- **Peer ID** (identificatore univoco P2P)
- **Nickname** (nome scelto per il contatto)
- **Stato online** (online/offline)
- **Ultimo accesso** (timestamp)
- **Avatar** (opzionale)

### 2.3 Dati di Comunicazione
- **Messaggi** (scambiati via P2P, **non memorizzati su nostri server**)
- **Cronologia peer** (peer scoperto in passato, memorizzato localmente)
- **Richieste di contatto** (pendenti, accettate, rifiutate)

### 2.4 Dati Tecnici
- **Indirizzo IP** (durante connessione P2P)
- **Log di sessione** (per debug/supporto)
- **Dati di configurazione** (impostazioni locali)

## 3. Base Legale

Trattiamo i tuoi dati sulla base di:
- **Consenso** (OAuth - esplicito durante login)
- **Esecuzione del contratto** (fornitura del servizio)
- **Legittimo interesse** (miglioramento servizio, sicurezza)

## 4. Come Utilizziamo i Dati

| Dato | Finalità | Durata |
|------|----------|--------|
| Email | Identificazione, sincronizzazione contatti via Drive | Fino al logout |
| Contatti | Comunicazione P2P | Fino eliminazione manuale |
| Messaggi | Comunicazione (solo tu e il destinatario) | Non memorizzati su server |
| Log tecnici | Debug, miglioramento sicurezza | 30 giorni |
| Cronologia peer | Ricerca veloce contatti | Fino a eliminazione manuale |

## 5. Archiviazione e Sicurezza

### 5.1 Archiviazione Locale
- **Percorso:** `~/.otter/` (directory nascosta)
- **Permessi:** 0600 (solo lettura/scrittura proprietario)
- **File:** 
  - `.identity.json` - Identità verificata su Google Drive
  - `.peers_discovered` - Cronologia peer (crittografia consigliata)
  - `contacts.json` - Contatti sincronizzati con Drive

### 5.2 Google Drive
- **Cartella:** `.otter/` (privata, solo tuo account)
- **File:** 
  - `identity.json` - Identità persistente
  - `contacts.json` - Lista contatti sincronizzata

### 5.3 Rete P2P
- **Comunicazione:** Diretta tra dispositivi (libp2p + mDNS)
- **Crittografia:** [Inserire dettagli: TLS, end-to-end encryption, ecc.]
- **No Storage Centrale:** Nessun server intermedio

### 5.4 Misure di Sicurezza
- ✅ PKCE OAuth2 (protezione token)
- ✅ Permessi 0600 su file locali (Unix)
- ✅ HTTPS per comunicazione esterna
- ✅ Nessuna password memorizzata
- ✅ Token autenticazione non persistenti

## 6. Condivisione di Dati

**NON condividiamo** i tuoi dati con:
- ❌ Aziende terze
- ❌ Servizi di tracciamento
- ❌ Reti pubblicitarie
- ❌ Ricercatori

**Condividiamo SOLO con:**
- ✅ Google (autenticazione OAuth + Drive - per tua scelta)
- ✅ Peer contatti (messaggi diretti via P2P)

## 7. I Tuoi Diritti

Hai il diritto di:
- **Accesso:** Richiedere copia dei tuoi dati
- **Rettifica:** Modificare dati inesatti
- **Cancellazione:** Eliminare profilo e dati ("diritto all'oblio")
- **Portabilità:** Esportare dati in formato leggibile
- **Revoca:** Revocare il consenso OAuth in qualsiasi momento
- **Opposizione:** Opporsi al trattamento per specifiche finalità

**Come esercitare:** Contatta: [email_supporto@otter.app]

## 8. Periodo di Conservazione

| Dato | Conservazione |
|------|----------------|
| Identità (Drive) | Fino eliminazione manuale |
| Contatti (locale + Drive) | Fino eliminazione manuale |
| Messaggi | Non memorizzati (P2P ephemeral) |
| Log tecnici | 30 giorni |
| Account annullato | 90 giorni (poi eliminato) |

## 9. GDPR e Conformità

- ✅ **GDPR:** Conforme (se interessati in UE)
- ✅ **Data Protection Officer:** [Inserire contatti]
- ✅ **Diritto di ricorso:** Autorità Garante Privacy [Paese]
- ✅ **Privacy by Design:** Minimizzazione dati da design

## 10. Modifiche a Questa Informativa

Possiamo aggiornare questa informativa periodicamente. Le modifiche significative saranno comunicate via:
- 📧 Email agli utenti registrati
- 📱 Notifica in-app

La data di ultimo aggiornamento è indicata sopra.

## 11. Contatti

**Titolare del Trattamento:**
```
Nome: [Inserire]
Email: [privacy@otter.app]
Address: [Indirizzo fiscale]
```

**Data Protection Officer (DPO):**
```
Email: dpo@otter.app
```

---

## Allegato A: Cookie e Tracking

**L'App NON utilizza:**
- ❌ Cookie di tracciamento
- ❌ Google Analytics
- ❌ Pixel tracking
- ❌ Social media pixels

**L'App utilizza:**
- ✅ Local Storage (credenziali OAuth, preferenze locali)
- ✅ Session Token (temporaneo, per sessione corrente)

---

## Allegato B: Specifiche Tecniche P2P

### Protocolli
- **libp2p:** Comunicazione distribuita
- **mDNS:** Discovery peer locale
- **DHT:** Distributed Hash Table (peer remoti)

### Dati Raccolti dalla Rete
- Peer ID (hash pubblico del certificato)
- Indirizzo IP (solo durante connessione attiva)
- Timestamp connessione

### Nessun Logging Centrale
Ogni dispositivo mantiene log locali. Non esiste server centrale che registra comunicazioni.

---

**Versione:** 1.0  
**Lingua:** Italiano (IT)  
**Giurisdizione:** [Italia/GDPR]

*Se hai domande sulla nostra informativa sulla privacy, contattaci a: privacy@otter.app*
