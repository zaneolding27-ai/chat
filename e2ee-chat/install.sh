#!/bin/bash
set -e

echo "=== Setting up Node.js 20 on Amazon Linux 2023 ==="
curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
sudo dnf install -y nodejs

echo "=== Setting up MongoDB 7.0 ==="
cat <<EOF | sudo tee /etc/yum.repos.d/mongodb-org-7.0.repo
[mongodb-org-7.0]
name=MongoDB Repository
baseurl=https://repo.mongodb.org/yum/amazon/2023/mongodb-org/7.0/x86_64/
gpgcheck=1
enabled=1
gpgkey=https://pgp.mongodb.com/server-7.0.asc
EOF
sudo dnf install -y mongodb-org
sudo systemctl enable mongod --now

echo "=== Installing npm dependencies ==="
npm install

echo "=== Starting server on port 3000 (MongoDB already running) ==="
echo "Replace DISCORD_BOT_TOKEN and LOG_CHANNEL_ID in server.js first!"
node server.js