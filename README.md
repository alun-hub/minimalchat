# MinimalChat

En minimal Mattermost-klient optimerad för långsamma satellit-förbindelser. Byggd med Tauri (Rust + HTML/CSS/JavaScript).

## Funktioner

✅ **Minimal nätverkstrafik**
- Komprimerad kommunikation med gzip
- Konfigurerbar synkroniseringsintervall
- Lokal SQLite-cache för offline-åtkomst
- Intelligent message queuing

✅ **Offline-stöd**
- Läs gamla meddelanden utan anslutning
- Skriv meddelanden som skickas automatiskt när anslutningen är tillbaka
- Lokal cache av kanaler och meddelanden

✅ **Optimerad för Windows**
- Litet minnesfotavtryck (~3-5 MB)
- Använder Windows native WebView
- Minimal underhållsbehov
- Stabil över lång tid

## Installation

### Förutsättningar

1. **Rust** - Installera från https://rustup.rs/
2. **Node.js** (16+) - Installera från https://nodejs.org/

### Steg 1: Installera dependencies

```bash
# Installera Node.js dependencies
npm install

# Tauri dependencies installeras automatiskt vid första build
```

### Steg 2: Utveckling (Dev mode)

```bash
npm run dev
```

Detta startar applikationen i utvecklingsläge med hot-reload.

### Steg 3: Bygg för produktion

```bash
npm run build
```

Detta skapar en Windows .exe-fil i `src-tauri/target/release/bundle/`.

## Användning

### Första anslutningen

1. Starta applikationen
2. Ange din Mattermost server URL (t.ex. `https://mattermost.example.com`)
3. Ange din Personal Access Token

#### Skapa Personal Access Token i Mattermost:

1. Logga in på Mattermost via webbläsaren
2. Gå till **Account Settings** → **Security** → **Personal Access Tokens**
3. Klicka på **Create Token**
4. Ge den ett namn (t.ex. "MinimalChat")
5. Kopiera token och klistra in i applikationen

### Använda applikationen

1. **Välj kanal** - Klicka på en kanal i vänsterpanelen
2. **Läs meddelanden** - Meddelanden laddas automatiskt
3. **Skicka meddelande** - Skriv i textfältet och klicka "Skicka" (eller Enter)
4. **Inställningar** - Klicka på kugghjulet för att justera synkroniseringsintervall

### Inställningar

- **Synkroniseringsintervall** - Hur ofta nya meddelanden hämtas (standard: 5 sekunder)
  - Högre värde = mindre nätverkstrafik
  - Lägre värde = snabbare uppdateringar

- **Max meddelanden per kanal** - Antal meddelanden att ladda (standard: 100)
  - Färre meddelanden = mindre nätverkstrafik

## Optimering för satellitförbindelser

### Rekommenderade inställningar:

- **Synkroniseringsintervall**: 10-30 sekunder för satellitförbindelser
- **Max meddelanden**: 50-100 beroende på bandbredd

### Så fungerar optimeringen:

1. **Gzip-komprimering** - All HTTP-trafik komprimeras automatiskt
2. **Lokal cache** - SQLite-databas lagrar allt lokalt
3. **Delta sync** - Hämtar bara nya meddelanden
4. **Message queuing** - Buffrar utgående meddelanden vid dålig anslutning
5. **Ingen WebSocket-polling** - Använder timer-baserad sync istället

## Felsökning

### "Failed to connect"

- Kontrollera att server URL är korrekt (med https://)
- Verifiera att access token är giltig
- Testa anslutningen i webbläsaren först

### Meddelanden uppdateras inte

- Kontrollera anslutningsstatus (grönt = online, rött = offline)
- Justera synkroniseringsintervallet i inställningar
- Klicka på 🔄 för manuell uppdatering

### Applikationen startar inte

- Kontrollera att Rust och Node.js är installerade
- Kör `npm install` igen
- Kontrollera konsolloggar för felmeddelanden

## Teknisk information

### Arkitektur

**Backend (Rust):**
- `main.rs` - Tauri huvudfil med kommandohanterare
- `mattermost.rs` - API-klient för Mattermost
- `database.rs` - SQLite-hantering för cache
- `models.rs` - Datamodeller

**Frontend (HTML/CSS/JS):**
- `index.html` - UI struktur
- `styles.css` - Styling
- `app.js` - Logik och Tauri-integration

**Databas:**
- SQLite med tabeller för channels, messages, settings, message_queue
- Automatisk indexering för snabba queries

### Mattermost API

Applikationen använder Mattermost REST API v4:
- `GET /api/v4/users/me` - Hämta användarinfo
- `GET /api/v4/users/{user_id}/teams/{team_id}/channels` - Hämta kanaler
- `GET /api/v4/channels/{channel_id}/posts` - Hämta meddelanden
- `POST /api/v4/posts` - Skicka meddelande

### Framtida förbättringar

- [ ] WebSocket-stöd för realtidsuppdateringar (med fallback)
- [ ] Stöd för filer och bilder (med miniatyrbilder)
- [ ] Notifikationer för nya meddelanden
- [ ] Flera team-stöd
- [ ] Emoji-stöd
- [ ] Markdown-rendering
- [ ] Sökning i meddelanden

## Licens

MIT

## Support

För frågor eller problem, kontakta utvecklaren eller öppna en issue på GitHub.
