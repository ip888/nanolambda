#!/bin/bash
set -e

echo "=== Deploying NanoLambda to DigitalOcean ==="

# Configuration
DROPLET_IP="${DO_HOST:-your-droplet-ip}"
DROPLET_USER="${DO_USERNAME:-root}"

echo "Deploying to: $DROPLET_USER@$DROPLET_IP"

# Build backend locally (optional - GitHub Actions will do this)
echo "Building backend..."
cargo build --release --bin nanolambda-server

# Deploy backend binary
echo "Deploying backend binary..."
scp target/release/nanolambda-server $DROPLET_USER@$DROPLET_IP:/tmp/nanolambda-server

ssh $DROPLET_USER@$DROPLET_IP << 'ENDSSH'
  # Stop old service
  systemctl stop nanolambda-server || true
  
  # Move new binary
  mv /tmp/nanolambda-server /usr/local/bin/nanolambda-server
  chmod +x /usr/local/bin/nanolambda-server
  
  # Create systemd service if it doesn't exist
  if [ ! -f /etc/systemd/system/nanolambda-server.service ]; then
    cat > /etc/systemd/system/nanolambda-server.service << 'EOF'
[Unit]
Description=NanoLambda Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/nanolambda
Environment="DATABASE_PATH=/var/lib/nanolambda/nanolambda.db"
Environment="RUST_LOG=info,nanolambda=debug"
ExecStart=/usr/local/bin/nanolambda-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
    systemctl enable nanolambda-server
  fi
  
  # Start service
  systemctl start nanolambda-server
  systemctl status nanolambda-server
ENDSSH

# Build and deploy frontend
echo "Building frontend..."
cd website
npm ci
NEXT_PUBLIC_API_URL=https://api.nanolambda.com npm run build

echo "Deploying frontend..."
rsync -avz --delete .next package.json package-lock.json public/ $DROPLET_USER@$DROPLET_IP:/var/www/nanolambda/

ssh $DROPLET_USER@$DROPLET_IP << 'ENDSSH'
  cd /var/www/nanolambda
  npm ci --production
  pm2 restart nanolambda-website || pm2 start npm --name nanolambda-website -- start
  pm2 save
ENDSSH

echo "=== Deployment Complete ==="
echo "Backend: https://api.nanolambda.com"
echo "Frontend: https://nanolambda.com"
