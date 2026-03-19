# E2EE Chat System (Proof of Concept)

Minimal end-to-end encrypted chat using Socket.IO + client-side crypto stub + Discord audit logging of every action.

**Security status**: educational / proof-of-concept only  
→ Current "encryption" is just Base64 — **not secure**  
→ Replace `encrypt()`/`decrypt()` with real cryptography (libsignal-protocol-javascript, @openpgpjs, etc.)

## Features
- Email + bcrypt registration / login
- JWT + Socket.IO auth
- Every action logged to private Discord channel (IP, UA, device, timestamp)
- Client-side encryption only (server sees only ciphertext)
- Single "general" room
- Vanilla HTML + JS frontend (no build step)

## Quick setup (Amazon Linux 2023 / most Linux servers)

```bash
git clone https://github.com/YOUR_USERNAME/e2ee-chat.git
cd e2ee-chat

# Edit server.js → replace DISCORD_BOT_TOKEN and LOG_CHANNEL_ID
chmod +x install.sh
./install.sh