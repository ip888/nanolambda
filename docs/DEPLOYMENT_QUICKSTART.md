# Nanolambda Deployment Quickstart

**⚡ 15-Minute Production Setup**

This is a condensed version of the comprehensive [Production Deployment Guide](PRODUCTION_DEPLOYMENT.md). Follow these steps for a basic production deployment.

> **Cloud Providers**: See the [Cloud Deployments](#cloud-deployments) section below for AWS, Digital Ocean, GCP, Linode, Hetzner, and Vultr specific instructions.

---

## Prerequisites

```bash
# Ubuntu 22.04/24.04 LTS
sudo apt update && sudo apt upgrade -y

# Install core dependencies
sudo apt install -y build-essential curl git nginx certbot python3-certbot-nginx sqlite3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install runtimes
sudo apt install -y python3 python3-pip  # Python
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs  # Node.js
```

---

## Quick Deploy

### 1. Create User & Directories

```bash
sudo useradd -r -s /bin/bash -d /opt/nanolambda -m nanolambda
sudo mkdir -p /var/lib/nanolambda/{data,logs,functions}
sudo chown -R nanolambda:nanolambda /var/lib/nanolambda
```

### 2. Build & Install

```bash
sudo -u nanolambda git clone https://github.com/ip888/nanolambda.git /opt/nanolambda/nanolambda
cd /opt/nanolambda/nanolambda
sudo -u nanolambda cargo build --release --bin server
sudo cp target/release/server /usr/local/bin/nanolambda-server
```

### 3. Create systemd Service

```bash
sudo tee /etc/systemd/system/nanolambda.service > /dev/null <<'EOF'
[Unit]
Description=Nanolambda Serverless Platform
After=network.target

[Service]
Type=simple
User=nanolambda
Group=nanolambda
WorkingDirectory=/var/lib/nanolambda
Environment="RUST_LOG=info"
Environment="NANOLAMBDA_HOST=127.0.0.1"
Environment="NANOLAMBDA_PORT=3000"
Environment="NANOLAMBDA_DB_PATH=/var/lib/nanolambda/data/nanolambda.db"
ExecStart=/usr/local/bin/nanolambda-server
Restart=always
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now nanolambda
sudo systemctl status nanolambda
```

### 4. Configure Nginx

```bash
sudo tee /etc/nginx/sites-available/nanolambda > /dev/null <<'EOF'
upstream nanolambda {
    server 127.0.0.1:3000;
    keepalive 32;
}

server {
    listen 80;
    server_name nanolambda.yourdomain.com;

    location / {
        proxy_pass http://nanolambda;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/nanolambda /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

### 5. Enable TLS (Let's Encrypt)

```bash
sudo certbot --nginx -d nanolambda.yourdomain.com
```

### 6. Setup Firewall

```bash
sudo ufw enable
sudo ufw allow 22/tcp   # SSH
sudo ufw allow 80/tcp   # HTTP
sudo ufw allow 443/tcp  # HTTPS
sudo ufw status
```

---

## Verify Installation

```bash
# Check service
sudo systemctl status nanolambda

# Test health endpoint
curl http://localhost:3000/health

# View logs
sudo journalctl -u nanolambda -f
```

---

## Quick Backup Setup

```bash
sudo tee /usr/local/bin/nanolambda-backup.sh > /dev/null <<'EOF'
#!/bin/bash
BACKUP_DIR="/var/backups/nanolambda"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p "$BACKUP_DIR"
sudo -u nanolambda sqlite3 /var/lib/nanolambda/data/nanolambda.db ".backup '$BACKUP_DIR/nanolambda_$DATE.db'"
gzip "$BACKUP_DIR/nanolambda_$DATE.db"
find "$BACKUP_DIR" -name "*.db.gz" -mtime +30 -delete
EOF

sudo chmod +x /usr/local/bin/nanolambda-backup.sh

# Add to crontab
echo "0 2 * * * /usr/local/bin/nanolambda-backup.sh" | sudo crontab -
```

---

## Quick Monitoring Setup

```bash
# Install Prometheus & Grafana
sudo apt install -y prometheus grafana

# Configure Prometheus
sudo tee -a /etc/prometheus/prometheus.yml > /dev/null <<'EOF'
  - job_name: 'nanolambda'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: /metrics
EOF

sudo systemctl restart prometheus
sudo systemctl enable --now grafana-server

# Access Grafana at http://localhost:3000 (admin/admin)
# Access Grafana at http://localhost:3000 (admin/admin)
```

---

## Cloud Deployments

### AWS EC2
```bash
# 1. Launch t3.medium instance (Ubuntu 24.04)
# 2. Configure security group: 22 (SSH), 80 (HTTP), 443 (HTTPS)
# 3. Connect and follow installation above
# 4. Use S3 for backups: aws s3 sync /var/backups/nanolambda s3://your-bucket/
```

### Digital Ocean
```bash
# 1. Create Basic 4GB Droplet (Ubuntu 24.04) - $24/month
# 2. Enable monitoring and backups
# 3. Add floating IP for stability
# 4. Follow installation above
# 5. Use Spaces for backups: s3cmd sync /var/backups/ s3://your-space/
```

### Google Cloud Platform
```bash
# 1. Create e2-medium VM (Ubuntu 24.04) - $27/month
# 2. Configure firewall rules
# 3. Follow installation above
# 4. Use Cloud Storage: gsutil rsync /var/backups/ gs://your-bucket/
```

### Hetzner Cloud (Best Price/Performance)
```bash
# 1. Create CX31 server (8GB RAM) - ~$12/month
# 2. Add SSH key
# 3. Follow installation above
# 4. Excellent price for European deployments
```

See the full [Cloud Provider Deployments](PRODUCTION_DEPLOYMENT.md#cloud-provider-deployments) guide for:
- Load balancer setup
- Managed databases
- Auto-scaling
- Cost optimization
- Provider-specific features

---

## What's Next?

✅ **Basic Deployment**: Complete!  
📚 **Full Guide**: See [PRODUCTION_DEPLOYMENT.md](PRODUCTION_DEPLOYMENT.md) for:
- Advanced security hardening
- Performance tuning
- Horizontal scaling
- Alert configuration
- Troubleshooting

---

## Common Commands

```bash
# Service management
sudo systemctl start nanolambda
sudo systemctl stop nanolambda
sudo systemctl restart nanolambda
sudo systemctl status nanolambda

# Logs
sudo journalctl -u nanolambda -f
sudo journalctl -u nanolambda -n 100

# Backup
sudo /usr/local/bin/nanolambda-backup.sh

# Health check
curl http://localhost:3000/health
```

---

## Need Help?

- 📖 **Full Documentation**: [PRODUCTION_DEPLOYMENT.md](PRODUCTION_DEPLOYMENT.md)
- 🐛 **Issues**: https://github.com/ip888/nanolambda/issues
- 💬 **Discussions**: https://github.com/ip888/nanolambda/discussions

---

**Time to First Function**: < 20 minutes! 🚀
