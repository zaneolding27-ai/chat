let socket;
let currentEmail = '';

function showChat() {
  document.getElementById('login').style.display = 'none';
  document.getElementById('chat').style.display = 'block';
}

function register() {
  const email = document.getElementById('email').value;
  const password = document.getElementById('password').value;
  fetch('/register', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  })
  .then(r => r.json())
  .then(data => {
    if (data.success) {
      document.getElementById('status').textContent = 'Registered! Now login.';
    } else {
      document.getElementById('status').textContent = data.error || 'Error';
    }
  });
}

function login() {
  const email = document.getElementById('email').value;
  const password = document.getElementById('password').value;
  fetch('/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  })
  .then(r => r.json())
  .then(data => {
    if (data.token) {
      currentEmail = data.email;
      localStorage.setItem('token', data.token);
      showChat();
      initSocket();
    } else {
      document.getElementById('status').textContent = data.error || 'Login failed';
    }
  });
}

function initSocket() {
  socket = io({
    auth: { token: localStorage.getItem('token') }
  });

  socket.on('connect', () => {
    socket.emit('joinRoom', 'general');
  });

  socket.on('receiveMessage', (msg) => {
    const decrypted = decrypt(msg.ciphertext); // E2EE stub
    const li = document.createElement('li');
    li.innerHTML = `<strong>${msg.from}</strong>: ${decrypted} <small style="color:#666">(${new Date(msg.time).toLocaleTimeString()})</small>`;
    document.getElementById('messages').appendChild(li);
    li.scrollIntoView();
  });

  socket.on('system', (text) => {
    const li = document.createElement('li');
    li.className = 'system';
    li.textContent = text;
    document.getElementById('messages').appendChild(li);
  });
}

function sendMessage() {
  const text = document.getElementById('msgInput').value.trim();
  if (!text || !socket) return;
  
  const ciphertext = encrypt(text); // E2EE stub
  socket.emit('sendMessage', { ciphertext });
  document.getElementById('msgInput').value = '';
}

// === E2EE STUB (Signal Protocol placeholder) ===
// In production replace with real libs (libsignal, @noble/curves, etc.)
// Server NEVER sees plaintext - only ciphertext below
function encrypt(plaintext) {
  // Demo: simple Base64 (replace with real AES-GCM + ratchet)
  return btoa(plaintext); // ← THIS IS NOT REAL E2EE
}

function decrypt(ciphertext) {
  return atob(ciphertext);
}

function logout() {
  localStorage.removeItem('token');
  location.reload();
}

// Auto-login if token exists
if (localStorage.getItem('token')) {
  currentEmail = 'restored';
  showChat();
  initSocket();
}