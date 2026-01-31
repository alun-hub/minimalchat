const { invoke } = window.__TAURI__.core;

// State
let currentChannelId = null;
let syncInterval = null;

// DOM Elements
const loginScreen = document.getElementById('login-screen');
const chatScreen = document.getElementById('chat-screen');
const settingsScreen = document.getElementById('settings-screen');

const loginForm = document.getElementById('login-form');
const loginError = document.getElementById('login-error');
const serverUrlInput = document.getElementById('server-url');
const authTokenInput = document.getElementById('auth-token');

const channelsList = document.getElementById('channels-list');
const messagesContainer = document.getElementById('messages-container');
const channelNameHeader = document.getElementById('channel-name');
const messageInput = document.getElementById('message-input');
const sendBtn = document.getElementById('send-btn');
const refreshBtn = document.getElementById('refresh-btn');
const settingsBtn = document.getElementById('settings-btn');
const closeSettingsBtn = document.getElementById('close-settings-btn');

const settingsForm = document.getElementById('settings-form');
const syncIntervalInput = document.getElementById('sync-interval');
const maxMessagesInput = document.getElementById('max-messages');
const currentServerDiv = document.getElementById('current-server');
const settingsMessage = document.getElementById('settings-message');
const disconnectBtn = document.getElementById('disconnect-btn');
const refreshChannelsBtn = document.getElementById('refresh-channels-btn');

const connectionStatus = document.getElementById('connection-status');

// Initialize
async function init() {
    try {
        const settings = await invoke('get_settings');

        if (settings.server_url && settings.auth_token) {
            // Try to auto-connect
            serverUrlInput.value = settings.server_url;
            authTokenInput.value = settings.auth_token;

            try {
                await connectToServer(settings.server_url, settings.auth_token);
                showChatScreen();
                loadChannels();
                startSync();
            } catch (e) {
                // If auto-connect fails, stay on login screen
                console.log('Auto-connect failed:', e);
            }
        }
    } catch (e) {
        console.error('Init error:', e);
    }
}

// Login
loginForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    console.log('Login form submitted');

    const serverUrl = serverUrlInput.value.trim();
    const authToken = authTokenInput.value.trim();

    if (!serverUrl || !authToken) {
        showError(loginError, 'Fyll i alla fält');
        return;
    }

    // Show loading state
    const submitBtn = loginForm.querySelector('button[type="submit"]');
    const originalText = submitBtn.textContent;
    submitBtn.textContent = 'Ansluter...';
    submitBtn.disabled = true;

    try {
        console.log('Connecting to:', serverUrl);
        await connectToServer(serverUrl, authToken);
        console.log('Connection successful, switching to chat screen');
        showChatScreen();
        loadChannels();
        startSync();
    } catch (error) {
        console.error('Connection failed:', error);
        showError(loginError, String(error));
    } finally {
        submitBtn.textContent = originalText;
        submitBtn.disabled = false;
    }
});

async function connectToServer(serverUrl, authToken) {
    console.log('connectToServer called with URL:', serverUrl);
    try {
        console.log('Invoking connect_to_server...');
        const result = await invoke('connect_to_server', {
            serverUrl,
            token: authToken
        });
        console.log('connect_to_server result:', result);
        updateConnectionStatus(true);
        return result;
    } catch (error) {
        console.error('connect_to_server error:', error);
        updateConnectionStatus(false);
        throw error;
    }
}

// Load channels
async function loadChannels() {
    try {
        channelsList.innerHTML = '<div class="loading">Laddar kanaler...</div>';
        const channels = await invoke('get_channels');

        if (channels.length === 0) {
            channelsList.innerHTML = '<div class="loading">Inga kanaler hittades</div>';
            return;
        }

        channelsList.innerHTML = '';
        channels.forEach(channel => {
            const channelEl = document.createElement('div');
            channelEl.className = 'channel-item';
            channelEl.innerHTML = `<div class="channel-name"># ${channel.display_name}</div>`;
            channelEl.dataset.channelId = channel.id;
            channelEl.dataset.channelName = channel.display_name;

            channelEl.addEventListener('click', () => {
                selectChannel(channel.id, channel.display_name);
            });

            channelsList.appendChild(channelEl);
        });
    } catch (error) {
        console.error('Failed to load channels:', error);
        channelsList.innerHTML = '<div class="loading">Fel vid laddning av kanaler</div>';
    }
}

// Select channel
async function selectChannel(channelId, channelName) {
    currentChannelId = channelId;
    channelNameHeader.textContent = `# ${channelName}`;

    // Update active state
    document.querySelectorAll('.channel-item').forEach(el => {
        el.classList.remove('active');
        if (el.dataset.channelId === channelId) {
            el.classList.add('active');
        }
    });

    await loadMessages(channelId);
}

// Load messages
async function loadMessages(channelId) {
    try {
        messagesContainer.innerHTML = '<div class="loading">Laddar meddelanden...</div>';

        const settings = await invoke('get_settings');
        const messages = await invoke('get_messages', {
            channelId,
            limit: settings.max_messages_per_channel
        });

        if (messages.length === 0) {
            messagesContainer.innerHTML = '<div class="empty-state"><p>Inga meddelanden ännu</p></div>';
            return;
        }

        // Reverse to show oldest first
        messages.reverse();

        // Group messages into threads
        const threads = groupMessagesIntoThreads(messages);

        messagesContainer.innerHTML = '';
        threads.forEach(thread => {
            renderThread(thread);
        });

        // Scroll to bottom
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
    } catch (error) {
        console.error('Failed to load messages:', error);
        messagesContainer.innerHTML = '<div class="empty-state"><p>Fel vid laddning av meddelanden</p></div>';
    }
}

// Group messages into threads
function groupMessagesIntoThreads(messages) {
    const threads = [];
    const threadMap = new Map(); // root_id -> { root: message, replies: [] }
    const processedIds = new Set();

    // First pass: identify root messages and create thread structures
    for (const message of messages) {
        if (!message.root_id || message.root_id === '') {
            // This is a root message
            threadMap.set(message.id, { root: message, replies: [] });
        }
    }

    // Second pass: attach replies to their root messages
    for (const message of messages) {
        if (message.root_id && message.root_id !== '') {
            // This is a reply
            if (threadMap.has(message.root_id)) {
                threadMap.get(message.root_id).replies.push(message);
                processedIds.add(message.id);
            }
        }
    }

    // Third pass: build final thread list in order
    for (const message of messages) {
        if (processedIds.has(message.id)) {
            continue; // Skip replies, they're attached to their root
        }

        if (threadMap.has(message.id)) {
            threads.push(threadMap.get(message.id));
        } else if (!message.root_id || message.root_id === '') {
            // Standalone message (not in threadMap for some reason)
            threads.push({ root: message, replies: [] });
        } else {
            // Reply whose root wasn't found - show as standalone
            threads.push({ root: message, replies: [] });
        }
    }

    return threads;
}

// Render a thread (root message + replies)
function renderThread(thread) {
    const { root, replies } = thread;

    if (replies.length === 0) {
        // No replies, just render the message normally
        appendMessage(root, false);
    } else {
        // Create thread container
        const threadContainer = document.createElement('div');
        threadContainer.className = 'thread-container';

        // Add root message
        const rootEl = createMessageElement(root, false);
        threadContainer.appendChild(rootEl);

        // Add replies container
        const repliesContainer = document.createElement('div');
        repliesContainer.className = 'thread-replies';

        replies.forEach(reply => {
            const replyEl = createMessageElement(reply, true);
            repliesContainer.appendChild(replyEl);
        });

        threadContainer.appendChild(repliesContainer);
        messagesContainer.appendChild(threadContainer);
    }
}

// Create message element
function createMessageElement(message, isReply = false) {
    const messageEl = document.createElement('div');
    messageEl.className = isReply ? 'message reply' : 'message';
    messageEl.dataset.messageId = message.id;

    const date = new Date(message.create_at);
    const timeStr = date.toLocaleTimeString('sv-SE', {
        hour: '2-digit',
        minute: '2-digit'
    });

    let replyIndicator = '';
    if (!isReply && message.reply_count > 0) {
        replyIndicator = `<div class="reply-count">${message.reply_count} svar</div>`;
    }

    messageEl.innerHTML = `
        <div class="message-header">
            <span class="message-user">${escapeHtml(message.username || message.user_id.substring(0, 8))}</span>
            <span class="message-time">${timeStr}</span>
        </div>
        <div class="message-text">${escapeHtml(message.message)}</div>
        ${replyIndicator}
    `;

    return messageEl;
}

// Append message to UI (for standalone messages)
function appendMessage(message, isReply = false) {
    const messageEl = createMessageElement(message, isReply);
    messagesContainer.appendChild(messageEl);
}

// Send message
sendBtn.addEventListener('click', sendMessage);
messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
    }
});

async function sendMessage() {
    const message = messageInput.value.trim();

    if (!message || !currentChannelId) {
        return;
    }

    try {
        await invoke('send_message', {
            channelId: currentChannelId,
            message
        });

        messageInput.value = '';

        // Refresh messages
        setTimeout(() => {
            if (currentChannelId) {
                loadMessages(currentChannelId);
            }
        }, 500);
    } catch (error) {
        console.error('Failed to send message:', error);
        alert('Kunde inte skicka meddelande: ' + error);
    }
}

// Refresh messages
refreshBtn.addEventListener('click', async () => {
    if (currentChannelId) {
        await loadMessages(currentChannelId);
    }
});

// Refresh channels
refreshChannelsBtn.addEventListener('click', async () => {
    await loadChannels();
});

// Settings
settingsBtn.addEventListener('click', async () => {
    showSettingsScreen();

    try {
        const settings = await invoke('get_settings');
        syncIntervalInput.value = settings.sync_interval_seconds;
        maxMessagesInput.value = settings.max_messages_per_channel;
        currentServerDiv.textContent = settings.server_url;
    } catch (error) {
        console.error('Failed to load settings:', error);
    }
});

closeSettingsBtn.addEventListener('click', () => {
    showChatScreen();
});

settingsForm.addEventListener('submit', async (e) => {
    e.preventDefault();

    try {
        const currentSettings = await invoke('get_settings');

        const newSettings = {
            ...currentSettings,
            sync_interval_seconds: parseInt(syncIntervalInput.value),
            max_messages_per_channel: parseInt(maxMessagesInput.value)
        };

        await invoke('save_settings', { settings: newSettings });

        showSuccess(settingsMessage, 'Inställningar sparade!');

        // Restart sync with new interval
        startSync();

        setTimeout(() => {
            showChatScreen();
        }, 1500);
    } catch (error) {
        showError(settingsMessage, 'Kunde inte spara: ' + error);
    }
});

disconnectBtn.addEventListener('click', () => {
    stopSync();
    showLoginScreen();
    currentChannelId = null;
    channelsList.innerHTML = '';
    messagesContainer.innerHTML = '<div class="empty-state"><p>Välj en kanal för att börja chatta</p></div>';
});

// Auto-sync
function startSync() {
    stopSync();

    invoke('get_settings').then(settings => {
        const interval = settings.sync_interval_seconds * 1000;

        syncInterval = setInterval(async () => {
            if (currentChannelId) {
                try {
                    // Sync queued messages first
                    await invoke('sync_queued_messages');

                    // Then refresh current channel
                    await loadMessages(currentChannelId);
                    updateConnectionStatus(true);
                } catch (error) {
                    console.error('Sync error:', error);
                    updateConnectionStatus(false);
                }
            }
        }, interval);
    });
}

function stopSync() {
    if (syncInterval) {
        clearInterval(syncInterval);
        syncInterval = null;
    }
}

// UI helpers
function showLoginScreen() {
    loginScreen.classList.add('active');
    chatScreen.classList.remove('active');
    settingsScreen.classList.remove('active');
}

function showChatScreen() {
    loginScreen.classList.remove('active');
    chatScreen.classList.add('active');
    settingsScreen.classList.remove('active');
}

function showSettingsScreen() {
    loginScreen.classList.remove('active');
    chatScreen.classList.remove('active');
    settingsScreen.classList.add('active');
}

function showError(element, message) {
    element.textContent = message;
    element.classList.add('show');
    setTimeout(() => element.classList.remove('show'), 5000);
}

function showSuccess(element, message) {
    element.textContent = message;
    element.classList.add('show');
    setTimeout(() => element.classList.remove('show'), 3000);
}

function updateConnectionStatus(online) {
    const dot = connectionStatus.querySelector('.status-dot');
    const text = connectionStatus.querySelector('span:last-child');

    if (online) {
        dot.classList.add('online');
        dot.classList.remove('offline');
        text.textContent = 'Ansluten';
    } else {
        dot.classList.add('offline');
        dot.classList.remove('online');
        text.textContent = 'Offline';
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Start app
init();
