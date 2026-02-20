# Google OAuth Setup - Otter App

## ✅ Cosa è cambiato: PKCE Flow (No Secret!)

L'app ora usa **PKCE (Proof Key for Code Exchange)** per l'autenticazione OAuth2. Questo significa:

- ✅ **Solo Client ID** è necessario (niente Client Secret)
- ✅ **Più sicuro** per app desktop e mobile
- ✅ **Distribuibile pubblicamente** - niente credenziali hardcoded
- ✅ **Niente configurazione per gli utenti** - click e accedi!

## 🚀 Per gli utenti finali

### Prima volta:
1. Apri l'app
2. Clicca "Login con Google"
3. Accedi al tuo account Google
4. ✅ Done! Sei dentro.

**Zero configurazione richiesta.**

---

## 🔧 Per i sviluppatori / Developer Setup

### Step 1: Crea un Google OAuth Project

1. Vai su [Google Cloud Console](https://console.cloud.google.com)
2. Seleziona o crea un nuovo progetto
3. Abilita le API:
   - Google Drive API
   - Google Identity API (o contatti)

### Step 2: Crea OAuth 2.0 Credentials

1. Vai a **Credenziali** → **Crea credenziali** → **Client ID OAuth**
2. Seleziona tipo: **Applicazione desktop**
3. Copia il **Client ID** (sarà come: `251946123352-7821m5nepc81d4u2k4d4nmv23dab14p0.apps.googleusercontent.com`)
4. **NON devi copiare il Secret** - PKCE non lo richiede!

### Step 3: Configura il `.env` locale

1. Copia `.env.example` in `.env`:
   ```bash
   cp .env.example .env
   ```

2. Sostituisci il valore:
   ```dotenv
   GOOGLE_CLIENT_ID=YOUR_CLIENT_ID.apps.googleusercontent.com
   ```

3. Salva e avvia con `cargo run`

### Step 4: (Opzionale) Configura Redirect URI in Google Console

Se vuoi essere preciso, aggiungi questa URI alla whitelist nelle credenziali OAuth:

```
http://localhost:8080/callback
```

(L'app comunque usa `localhost:8080` per ricevere il callback)

---

## 📦 Per la Distribuzione (Utenti Finali)

L'app distribuita può avere il **Client ID hardcoded** direttamente nel codice:

```rust
let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
    // Embedded Client ID - disponibile a tutti
    "251946123352-7821m5nepc81d4u2k4d4nmv23dab14p0.apps.googleusercontent.com".to_string()
});
```

**Perché è sicuro?**
- PKCE non richiede il ClientSecret
- Il ClientSecret rimane nel tuo server Google
- L'app non ha accesso al Secret

---

## 🔐 Note sulla Sicurezza

### PKCE (Proof Key for Code Exchange):

1. **code_verifier**: Stringa random generata dalla nostra app
2. **code_challenge**: Hash SHA256 del code_verifier
   - Inviamo il challenge a Google (NON il verifier)
   
3. Durante scambio token:
   - Google richiede il verifier
   - Verifica che SHA256(verifier) == challenge
   - Solo chi ha il verifier può scambiare il token
   
4. **Questo protegge da**:
   - Attacchi di intercettazione
   - Attacchi on HTTPS downgrade
   - Replay attacks

### Scope richiesti:
```
- https://www.googleapis.com/auth/userinfo.email
- https://www.googleapis.com/auth/userinfo.profile
- https://www.googleapis.com/auth/drive.file (lettura/scrittura contatti)
- openid
```

---

## ✨ Differenze: OAuth Standard vs. PKCE

### Prima (OAuth Standard):
```
App → Google: "Ciao, io sono [CLIENT_ID] con secret [CLIENT_SECRET]"
↓ (Google verifica il secret)
↓ 
Google → App: "Ok, ecco il token"
```
❌ Problema: Se il secret è pubblico, chiunque può impersonare l'app

---

### Adesso (PKCE):
```
App genera code_verifier (random)
App calcola code_challenge = SHA256(code_verifier)

App → Google: "Voglio autorizzare, ecco il challenge"
↓ (Google ricorda il challenge)
↓
Utente: "Si, autorizza!"
↓
App → Google: "Ecco il code_verifier"
↓ (Google verifica: SHA256(code_verifier) == challenge)
↓
Google → App: "Ok, ecco il token"
```
✅ Sicuro: Solo chi ha il verifier può scambiare il token!

---

## 🐛 Troubleshooting

### "Google OAuth non configurato"
- Verifica che GOOGLE_CLIENT_ID sia impostato in `.env`
- Non deve contenere `YOUR_CLIENT_ID`

### "Impossibile aprire il browser"
- Assicurati che `open` crate sia installato
- Su Linux: `sudo apt install xdg-open`

### "Porta 8080 occupata"
- Cambia la porta in `main.rs` (costante `REDIRECT_URI`)
- Aggiorna anche in Google Cloud Console

---

## 📚 Risorse

- [PKCE RFC 7636](https://tools.ietf.org/html/rfc7636)
- [Google OAuth2 Docs](https://developers.google.com/identity/protocols/oauth2)
- [oauth2-rs Library](https://github.com/ramosbugs/oauth2-rs)
