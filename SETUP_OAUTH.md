# Google OAuth Setup for Otter GUI

The Otter GUI uses Google OAuth2 for authentication and Google Drive for identity storage.

## Setup Instructions

### 1. Get Google OAuth Credentials

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the following APIs:
   - Google Drive API
   - Google+ API
4. Go to **Credentials** → **Create Credentials** → **OAuth 2.0 Client ID**
5. Set Application type to **Desktop app**
6. Note down your **Client ID** and **Client Secret**

### 2. Configure Authorized Redirect URIs

In the OAuth client configuration, add:
```
http://localhost:8080
```

### 3. Set Environment Variables

Before running the GUI, set these environment variables:

**Linux/macOS:**
```bash
export GOOGLE_CLIENT_ID="your-client-id.apps.googleusercontent.com"
export GOOGLE_CLIENT_SECRET="your-client-secret"
```

**Windows (PowerShell):**
```powershell
$env:GOOGLE_CLIENT_ID="your-client-id.apps.googleusercontent.com"
$env:GOOGLE_CLIENT_SECRET="your-client-secret"
```

**Windows (CMD):**
```cmd
set GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
set GOOGLE_CLIENT_SECRET=your-client-secret
```

### 4. Run Otter GUI

```bash
cargo run --bin otter
```

## Required OAuth Scopes

The application requests the following scopes:
- `https://www.googleapis.com/auth/userinfo.email` - Read user email
- `https://www.googleapis.com/auth/userinfo.profile` - Read user profile
- `https://www.googleapis.com/auth/drive.file` - Access app-created files on Drive
- `openid` - OpenID Connect authentication

## Security Notes

⚠️ **Never commit your OAuth credentials to version control!**

- Environment variables are the recommended method for production
- The `config.toml` file is git-ignored for safety
- Keep your Client Secret private and secure
