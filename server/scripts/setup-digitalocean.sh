#!/bin/bash
set -e

# DigitalOcean Droplet Setup Script
# Run this once on a fresh Ubuntu 24.04 droplet

echo "=== NanoLambda Deployment Setup ==="

# Update system
echo "Updating system packages..."
apt-get update
apt-get upgrade -y

# Install Docker
echo "Installing Docker..."
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
systemctl enable docker
systemctl start docker
rm get-docker.sh

# Install Docker Compose
echo "Installing Docker Compose..."
apt-get install -y docker-compose-plugin

# Install Node.js and PM2 (for frontend)
echo "Installing Node.js..."
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs
npm install -g pm2

# Create application directories
echo "Creating application directories..."
mkdir -p /var/lib/nanolambda
mkdir -p /var/www/nanolambda
chown -R $USER:$USER /var/lib/nanolambda
chown -R $USER:$USER /var/www/nanolambda

# Setup firewall
echo "Configuring firewall..."
ufw allow OpenSSH
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow 8080/tcp
ufw --force enable

# Install Caddy (reverse proxy with auto SSL)
echo "Installing Caddy..."
apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
apt update
apt install -y caddy

# Create Caddyfile
cat > /etc/caddy/Caddyfile << 'EOF'
# Replace with your domain
nanolambda.com {
    # Frontend
    reverse_proxy localhost:3000
}

api.nanolambda.com {
    # Backend API
    reverse_proxy localhost:8080
}
EOF

# Reload Caddy
systemctl reload caddy

# Setup PM2 to start on boot
pm2 startup systemd -u $USER --hp /home/$USER
pm2 save

echo "=== Setup Complete ==="
echo "Next steps:"
echo "1. Add GitHub SSH key to this server"
echo "2. Configure GitHub Secrets:"
echo "   - DO_HOST: $(curl -s ifconfig.me)"
echo "   - DO_USERNAME: $USER"
echo "   - DO_SSH_KEY: (your private key)"
echo "   - API_URL: https://api.nanolambda.com"
echo "3. Update Caddyfile with your actual domain"
echo "4. Push to GitHub to trigger deployment"
