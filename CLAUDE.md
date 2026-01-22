# CLAUDE.md - AI Assistant Guide for MinimalChat

## Project Overview

**MinimalChat** är en minimal Mattermost-klient optimerad för långsamma satellit-förbindelser. Projektet är byggt med **Tauri 2.0** (Rust backend + HTML/CSS/JavaScript frontend) för att minimera nätverkstrafik och minnesfotavtryck.

### Huvudmål
1. **Minimal nätverkstrafik** - Kritiskt för satellit-anslutningar
2. **Offline-stöd** - Fungera utan konstant anslutning
3. **Windows-optimerad** - Långsiktig stabilitet och minimalt underhåll
4. **Litet minnesfotavtryck** - ~3-5 MB vs Electron ~50+ MB

### Nyckelteknologier
- **Tauri 2.0** - Native Windows WebView, litet fotavtryck
- **Rust** - Backend med HTTP-klient, SQLite, säkerhet
- **SQLite** - Lokal cache/databas
- **Vanilla JavaScript** - Ingen framework overhead
- **Mattermost API v4** - REST API med gzip-komprimering

## Repository Structure

```
minimalchat/
├── src/                          # Frontend (HTML/CSS/JS)
│   ├── index.html                # Huvudvy med tre screens
│   ├── styles.css                # Dark theme styling
│   └── app.js                    # Frontend logik, Tauri integration
│
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs              # Tauri entry point, command handlers
│   │   ├── mattermost.rs        # Mattermost API client
│   │   ├── database.rs          # SQLite operations
│   │   └── models.rs            # Data structures
│   ├── Cargo.toml               # Rust dependencies
│   ├── tauri.conf.json          # Tauri configuration
│   ├── build.rs                 # Build script
│   └── icons/                   # App icons
│
├── package.json                 # Node.js dependencies
├── README.md                    # User documentation
├── .gitignore                   # Git ignore rules
└── CLAUDE.md                    # This file
```

## Key Files Deep Dive

### Backend (Rust)

#### `src-tauri/src/main.rs`
**Purpose**: Tauri application entry point och command handlers

**Key Functions**:
- `connect_to_server()` - Anslut till Mattermost server med token
- `get_channels()` - Hämta kanaler från server eller cache
- `get_messages()` - Hämta meddelanden från server eller cache
- `send_message()` - Skicka meddelande (eller queue vid offline)
- `get_settings()` / `save_settings()` - Hantera app-inställningar
- `sync_queued_messages()` - Synka köade meddelanden när online

**State Management**:
```rust
pub struct AppState {
    pub client: Option<MattermostClient>,  // API client (None when offline)
    pub db: Database,                       // SQLite connection
    pub settings: Settings,                 // User settings
}
```

**Important**: Alla commands är async och returnerar `Result<T, String>` för felhantering.

#### `src-tauri/src/mattermost.rs`
**Purpose**: HTTP-klient för Mattermost REST API

**Key Features**:
- Gzip-komprimering aktiverad (`client.gzip(true)`)
- Bearer token authentication
- Error handling med `Box<dyn Error>`

**API Endpoints Used**:
- `GET /api/v4/users/me` - Validera anslutning
- `GET /api/v4/users/{user_id}/teams/{team_id}/channels` - Hämta kanaler
- `GET /api/v4/channels/{channel_id}/posts?per_page={limit}` - Hämta meddelanden
- `POST /api/v4/posts` - Skicka meddelande

**Network Optimization**:
- Gzip compression reducerar bandbredd med ~70%
- Batch requests där möjligt
- Caching för att minimera API-anrop

#### `src-tauri/src/database.rs`
**Purpose**: SQLite-hantering för lokal cache och offline-stöd

**Tables**:
1. **channels** - Cache av användarens kanaler
   - Kolumner: id (PK), name, display_name, team_id, channel_type

2. **messages** - Cache av meddelanden
   - Kolumner: id (PK), channel_id, user_id, message, create_at
   - Index: (channel_id, create_at DESC) för snabba queries

3. **settings** - Key-value store för app-inställningar
   - Kolumner: key (PK), value

4. **message_queue** - Köa meddelanden vid offline
   - Kolumner: id (AUTOINCREMENT), channel_id, message, created_at

**Key Methods**:
- `save_channel()` / `get_channels()` - Cache channels
- `save_message()` / `get_messages()` - Cache messages
- `save_settings()` / `get_settings()` - Persist settings
- `queue_message()` / `get_queued_messages()` / `remove_queued_message()` - Offline queue

**Thread Safety**: Database använder `Arc<Mutex<Connection>>` för säker concurrent access.

#### `src-tauri/src/models.rs`
**Purpose**: Datastrukturer och serialization

**Models**:
```rust
pub struct Channel {
    pub id: String,
    pub name: String,           // Internal name (e.g., "town-square")
    pub display_name: String,   // Display name (e.g., "Town Square")
    pub team_id: String,
    pub channel_type: String,   // "O" (open), "P" (private), "D" (direct)
}

pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub user_id: String,
    pub message: String,
    pub create_at: i64,         // Unix timestamp in milliseconds
}

pub struct Settings {
    pub server_url: String,
    pub auth_token: String,
    pub sync_interval_seconds: u64,      // Default: 5
    pub max_messages_per_channel: i32,   // Default: 100
}
```

### Frontend (JavaScript)

#### `src/app.js`
**Purpose**: Frontend logik och Tauri integration

**Key State**:
```javascript
let currentChannelId = null;    // Currently selected channel
let syncInterval = null;        // Auto-sync interval timer
```

**Main Functions**:

1. **init()** - Auto-connect om saved credentials finns
2. **connectToServer()** - Anslut till Mattermost
3. **loadChannels()** - Ladda och rendera kanallista
4. **selectChannel()** - Byt aktiv kanal
5. **loadMessages()** - Ladda och rendera meddelanden
6. **sendMessage()** - Skicka meddelande
7. **startSync()** / **stopSync()** - Auto-sync timer

**Auto-Sync Mechanism**:
```javascript
// Synkar varje X sekunder (konfigurerbart)
syncInterval = setInterval(async () => {
    if (currentChannelId) {
        await invoke('sync_queued_messages');  // Skicka köade meddelanden först
        await loadMessages(currentChannelId);   // Uppdatera aktuell kanal
        updateConnectionStatus(true);
    }
}, interval);
```

**Tauri Integration**:
- Använder `window.__TAURI__.core.invoke()` för att anropa Rust commands
- Alla anrop är async och returnerar Promises
- Error handling med try/catch

#### `src/index.html`
**Purpose**: UI struktur med tre huvudscreens

**Screens**:
1. **login-screen** - Server URL + Access Token
2. **chat-screen** - Huvudvy med sidebar och chatarea
3. **settings-screen** - Inställningar

**Layout**:
```
chat-screen
├── sidebar (280px)
│   ├── sidebar-header (channels title + settings button)
│   ├── channels-list (scrollable)
│   └── sidebar-footer (connection status)
│
└── chat-area (flex: 1)
    ├── chat-header (channel name + refresh button)
    ├── messages-container (scrollable)
    └── message-input-container (textarea + send button)
```

#### `src/styles.css`
**Purpose**: Dark theme styling

**Design System**:
- Background: `#1e1e1e` (main), `#2d2d2d` (elevated)
- Text: `#e0e0e0` (main), `#ccc` (secondary), `#888` (muted)
- Accent: `#4a9eff` (blue)
- Success: `#6bff6b`, Error: `#ff6b6b`

**Responsive**: Min width 800px, min height 600px

## Development Workflows

### Initial Setup

```bash
# 1. Install dependencies
npm install

# 2. Run in dev mode (hot reload)
npm run dev

# 3. Build for production
npm run build
```

### Adding New Features

#### Adding a New Tauri Command

1. **Define in `main.rs`**:
```rust
#[tauri::command]
async fn my_new_command(
    param: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    // Implementation
    Ok("Success".to_string())
}
```

2. **Register in `invoke_handler`**:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    my_new_command,
])
```

3. **Call from frontend**:
```javascript
const result = await invoke('my_new_command', { param: 'value' });
```

#### Adding Database Fields

1. **Update `database.rs`**:
```rust
// Add migration in Database::new()
conn.execute(
    "ALTER TABLE tablename ADD COLUMN new_field TEXT",
    [],
)?;
```

2. **Update model in `models.rs`**:
```rust
pub struct MyModel {
    pub new_field: String,
}
```

3. **Update save/get methods** i `database.rs`

#### Adding UI Elements

1. **Update `index.html`** med ny markup
2. **Update `styles.css`** med styling
3. **Update `app.js`** med event handlers

### Testing

#### Manual Testing
```bash
# Dev mode with console logs
npm run dev

# Check Rust logs
RUST_LOG=debug npm run dev

# Build and test production build
npm run build
# Find .exe in src-tauri/target/release/bundle/
```

#### API Testing
```bash
# Test Mattermost API manually
curl -H "Authorization: Bearer YOUR_TOKEN" \
     https://your-server.com/api/v4/users/me
```

### Common Issues & Solutions

#### "Failed to connect"
- **Cause**: Invalid server URL or token
- **Fix**: Verify credentials, check network
- **Debug**: Check browser console for error details

#### "Database locked"
- **Cause**: Multiple simultaneous writes
- **Fix**: Ensure proper `Arc<Mutex<>>` usage
- **Debug**: Check for deadlocks in Rust code

#### Messages not syncing
- **Cause**: Offline or wrong sync interval
- **Fix**: Check connection status, adjust settings
- **Debug**: Monitor browser console during sync

## Conventions & Best Practices

### Code Style

#### Rust
- Use `rustfmt` for formatting
- Error handling: Return `Result<T, String>` from commands
- Async: All network operations are async
- Thread safety: Use `Arc<Mutex<>>` for shared state

#### JavaScript
- Use `async/await` for Tauri commands
- Escape HTML in user content (`escapeHtml()`)
- Handle errors with try/catch
- Use `const` by default, `let` when needed

### Naming Conventions

#### Rust
- `snake_case` for functions, variables
- `PascalCase` for types, structs
- File names: `lowercase_with_underscores.rs`

#### Frontend
- `camelCase` for JS functions, variables
- `kebab-case` for HTML/CSS classes
- `UPPER_CASE` for constants

### Security

1. **Never log tokens** - Sensitive data
2. **Escape HTML** - Prevent XSS
3. **Validate input** - Server-side validation
4. **Use HTTPS** - Always for Mattermost API

### Performance

1. **Minimize API calls** - Use cache first
2. **Batch operations** - Group database writes
3. **Lazy loading** - Load messages on demand
4. **Debounce** - Avoid rapid repeated calls

## Satellite Connection Optimization

### Current Optimizations

1. **Gzip Compression**
   - Enabled in reqwest client
   - ~70% bandwidth reduction
   - Automatic for all HTTP requests

2. **Local SQLite Cache**
   - All channels cached locally
   - Messages cached per channel
   - Settings persisted locally

3. **Configurable Sync Interval**
   - Default: 5 seconds
   - Recommended for satellite: 10-30 seconds
   - User-configurable via settings

4. **Message Queuing**
   - Outgoing messages queued when offline
   - Auto-sync when connection restored
   - Prevents message loss

5. **Delta Sync**
   - Only fetch new messages
   - Timestamp-based filtering
   - Reduces repeated data transfer

### Bandwidth Usage Estimates

**Initial Load** (first connection):
- User info: ~1 KB
- Channels (10): ~5 KB
- Messages (100/channel): ~50 KB
- **Total: ~56 KB (compressed: ~20 KB)**

**Per Sync** (every 5 seconds):
- New messages (1-5): ~0.5-2.5 KB
- **Compressed: ~0.2-1 KB**

**Hourly Usage** (5 sec interval, 5 channels):
- 720 syncs/hour × 1 KB = ~720 KB/hour
- **~17 MB/day** (active use, 10 hours/day)

### Future Optimizations

1. **WebSocket with Reconnection Logic**
   - Real-time updates when connected
   - Graceful fallback to polling
   - Connection keepalive tuning

2. **Differential Sync**
   - Send only message IDs first
   - Fetch full content for new messages
   - Reduces redundant transfers

3. **Image Compression**
   - Thumbnail generation
   - Progressive loading
   - Format conversion (WebP)

4. **Message Bundling**
   - Send multiple messages in one request
   - Receive multiple updates in one response
   - Reduces HTTP overhead

## Common Tasks for AI Assistants

### "Add support for [feature]"

1. **Analyze requirements** - Understand what's needed
2. **Check Mattermost API** - Verify API support
3. **Update models** - Add data structures
4. **Update database** - Add tables/columns if needed
5. **Implement backend** - Add Rust command
6. **Update frontend** - Add UI and logic
7. **Test** - Verify functionality

### "Fix bug with [feature]"

1. **Reproduce** - Understand the issue
2. **Check logs** - Browser console + Rust logs
3. **Identify root cause** - Frontend vs Backend
4. **Implement fix** - Make minimal changes
5. **Test** - Verify fix works
6. **Check side effects** - Ensure no regressions

### "Optimize [feature]"

1. **Measure current performance** - Baseline metrics
2. **Identify bottleneck** - Network, CPU, memory
3. **Implement optimization** - Cache, batch, compress
4. **Measure improvement** - Compare metrics
5. **Document** - Update this file if significant

### "Update dependencies"

```bash
# Update Rust dependencies
cd src-tauri
cargo update

# Update Node dependencies
npm update

# Check for outdated packages
cargo outdated
npm outdated
```

## Architecture Decisions

### Why Tauri over Electron?
- **Size**: 3-5 MB vs 50+ MB
- **Memory**: Uses native WebView
- **Performance**: Rust backend is faster
- **Security**: Better sandboxing
- **Maintenance**: Fewer dependencies

### Why SQLite over other databases?
- **Embedded**: No separate server
- **Fast**: Optimized for local reads
- **Reliable**: ACID compliant
- **Small**: Minimal overhead
- **Cross-platform**: Works everywhere

### Why Vanilla JS over frameworks?
- **Size**: No framework overhead
- **Simple**: Less complexity
- **Fast**: Direct DOM manipulation
- **Maintenance**: No framework updates

### Why REST API over WebSocket?
- **Initial**: Easier to implement
- **Reliable**: Better error handling
- **Future**: WebSocket kan läggas till senare
- **Bandwidth**: With sync interval, similar usage

## Troubleshooting Guide

### Build Issues

**"Cannot find Rust"**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**"Tauri command not found"**
```bash
npm install
```

**"Database error"**
```bash
# Delete database and restart
rm minimalchat.db
npm run dev
```

### Runtime Issues

**"White screen"**
- Check browser console for errors
- Verify `src/` directory exists
- Check `tauri.conf.json` paths

**"Cannot connect to server"**
- Verify server URL (must include https://)
- Verify access token is valid
- Check network connectivity
- Check CORS settings on server

**"Messages not loading"**
- Check connection status indicator
- Verify channel is selected
- Check sync interval settings
- Check browser console for errors

## Version History

- **0.1.0** (2026-01-22) - Initial implementation
  - Basic Mattermost integration
  - SQLite caching
  - Offline message queue
  - Configurable sync interval
  - Dark theme UI

## Next Steps

Recommended improvements in priority order:

1. **Icons** - Generate proper app icons
2. **User Display** - Show usernames instead of user IDs
3. **Markdown** - Render markdown in messages
4. **Notifications** - Desktop notifications for new messages
5. **WebSocket** - Real-time updates with fallback
6. **File Support** - Upload/download files
7. **Search** - Search messages locally
8. **Multi-team** - Support multiple teams
9. **Emoji** - Emoji picker and rendering

## References

- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Mattermost API Documentation](https://api.mattermost.com/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [SQLite Documentation](https://www.sqlite.org/docs.html)

---

**Last Updated**: 2026-01-22
**Maintained by**: AI Assistant + Developer
**Language**: Swedish (UI), English (code/docs)
