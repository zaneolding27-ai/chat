const express = require('express');
const helmet = require('helmet');
const cors = require('cors');
const rateLimit = require('express-rate-limit');
const mongoose = require('mongoose');
const bcrypt = require('bcryptjs');
const jwt = require('jsonwebtoken');
const { Server } = require('socket.io');
const http = require('http');
const UAParser = require('ua-parser-js');
const { Client, GatewayIntentBits } = require('discord.js');

const app = express();
const server = http.createServer(app);
const io = new Server(server, { cors: { origin: "*" } });

// ────────────────────────────────────────────────
//          ← REPLACE THESE WITH REAL VALUES →
const DISCORD_BOT_TOKEN     = 'YOUR_NEW_BOT_TOKEN_HERE_AFTER_RESET';   // ← Paste NEW token here
const LOG_CHANNEL_ID        = 'YOUR_PRIVATE_CHANNEL_ID_HERE';         // ← Your logging channel
const JWT_SECRET            = 'change-this-to-a-very-long-random-secret'; // ← Change this!
// ────────────────────────────────────────────────

const BOT_ID = '1484016485834686524';  // Your bot's user ID (extracted from old token)

// Mention format for Discord (makes logs ping/highlight the bot if needed)
const botMention = `<@${BOT_ID}>`;

const bot = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    // If you want the bot to read its own messages or be mentionable:
    GatewayIntentBits.MessageContent
  ]
});

let logChannel = null;

bot.once('ready', () => {
  logChannel = bot.channels.cache.get(LOG_CHANNEL_ID);
  if (logChannel) {
    console.log(`✅ Discord bot logged in as ${bot.user.tag} (ID: ${BOT_ID})`);
    console.log(`Logging messages to channel: #${logChannel.name || 'unknown'}`);
  } else {
    console.error(`❌ Could not find channel with ID ${LOG_CHANNEL_ID}`);
  }
});

bot.login(DISCORD_BOT_TOKEN).catch(err => {
  console.error('Discord login failed:', err.message);
});

const User = require('./models/User');

// Middleware
app.use(helmet());
app.use(cors({ origin: '*' }));
app.use(express.json());
app.use(express.static('public'));

const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 5,
  message: { error: 'Too many login attempts, try again later' }
});

// Logging helper – now includes bot mention for better visibility
async function logAction(action, userId, reqOrSocket, extra = '') {
  const uaString = reqOrSocket.headers 
    ? reqOrSocket.headers['user-agent'] 
    : (reqOrSocket.handshake?.headers['user-agent'] || '');

  const parser = new UAParser(uaString);
  const device = parser.getDevice().type || 'desktop';
  const browser = `${parser.getBrowser().name || 'Unknown'} ${parser.getBrowser().version || ''}`.trim();
  const ip = (reqOrSocket.ip || reqOrSocket.handshake?.address || 'unknown').replace('::ffff:', '');
  const timestamp = new Date().toISOString().replace('T', ' ').split('.')[0];

  const logMsg = `**${action}** ${botMention}\n` +
                 `• User: ${userId}\n` +
                 `• IP: ${ip}\n` +
                 `• Browser: ${browser}\n` +
                 `• Device: ${device}\n` +
                 `• Time: ${timestamp}${extra ? `\n• Extra: ${extra}` : ''}`;

  console.log(logMsg.replace(/\n/g, ' | ')); // console-friendly

  if (logChannel) {
    try {
      await logChannel.send(logMsg);
    } catch (err) {
      console.error('Failed to send to Discord:', err.message);
    }
  }
}

// ────────────────────────────────────────────────
// Routes (unchanged except logging calls)
// ────────────────────────────────────────────────

app.post('/register', async (req, res) => {
  try {
    const { email, password } = req.body;
    if (!email || !password) return res.status(400).json({ error: 'Missing fields' });

    if (await User.findOne({ email })) {
      return res.status(400).json({ error: 'User already exists' });
    }

    const hash = await bcrypt.hash(password, 10);
    await User.create({ email, password: hash });

    await logAction('User registered', email, req);
    res.json({ success: true, message: 'Account created – you can now log in' });
  } catch (e) {
    res.status(500).json({ error: e.message });
  }
});

app.post('/login', loginLimiter, async (req, res) => {
  try {
    const { email, password } = req.body;
    const user = await User.findOne({ email });

    if (!user || !(await bcrypt.compare(password, user.password))) {
      return res.status(401).json({ error: 'Invalid email or password' });
    }

    const token = jwt.sign({ email }, JWT_SECRET, { expiresIn: '2h' });

    await logAction('User login successful', email, req);
    res.json({ token, email });
  } catch (e) {
    res.status(500).json({ error: e.message });
  }
});

// Socket.IO middleware + events (unchanged logic)
io.use(async (socket, next) => {
  const token = socket.handshake.auth?.token;
  if (!token) return next(new Error('Authentication token required'));

  try {
    const payload = jwt.verify(token, JWT_SECRET);
    socket.data.email = payload.email;
    next();
  } catch (err) {
    next(new Error('Invalid or expired token'));
  }
});

io.on('connection', (socket) => {
  const email = socket.data.email;
  logAction('Socket connected', email, socket);

  socket.on('joinRoom', (room = 'general') => {
    socket.join(room);
    logAction('Joined room', email, socket, `room: ${room}`);
    socket.to(room).emit('system', `${email} joined the room`);
  });

  socket.on('sendMessage', (data) => {
    if (!data.ciphertext) return;
    logAction('Message sent', email, socket, `ciphertext length: ${data.ciphertext.length}`);
    io.to('general').emit('receiveMessage', {
      from: email,
      ciphertext: data.ciphertext,
      time: new Date().toISOString()
    });
  });

  socket.on('disconnect', () => {
    logAction('Socket disconnected', email, socket);
  });
});

// Start server
mongoose.connect('mongodb://127.0.0.1:27017/e2eechat', {
  serverSelectionTimeoutMS: 5000
})
  .then(() => {
    server.listen(3000, () => {
      console.log('🚀 Server running → http://0.0.0.0:3000');
      console.log('📡 Waiting for Discord bot to connect...');
    });
  })
  .catch(err => {
    console.error('MongoDB connection failed:', err.message);
    process.exit(1);
  });