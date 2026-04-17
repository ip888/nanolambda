# Nanolambda Production Deployment Guide

**Version**: 1.0  
**Date**: October 18, 2024  
**Status**: Production Ready  

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [System Requirements](#system-requirements)
4. [Installation](#installation)
5. [systemd Service Configuration](#systemd-service-configuration)
6. [Nginx Reverse Proxy](#nginx-reverse-proxy)
7. [TLS/SSL Configuration](#tlsssl-configuration)
8. [Database Setup](#database-setup)
9. [Monitoring Stack](#monitoring-stack)
10. [Security Hardening](#security-hardening)
11. [Backup & Recovery](#backup--recovery)
12. [Scaling Strategies](#scaling-strategies)
13. [Health Checks](#health-checks)
14. [Cloud Provider Deployments](#cloud-provider-deployments)
15. [Troubleshooting](#troubleshooting)
16. [Performance Tuning](#performance-tuning)

---

## Overview

This guide provides comprehensive instructions for deploying Nanolambda in production environments. It covers everything from initial setup to advanced monitoring and scaling strategies.

**Target Environment**: Ubuntu 22.04/24.04 LTS (adaptable to other Linux distributions)

### Architecture Overview

```
Internet
    ↓
[Nginx Reverse Proxy]
    ↓ (HTTP/HTTPS)
[Nanolambda API Server]
    ↓
┌─────────────────────────────────┐
│ Runtime Layer                   │
│ ├── PythonExecutor              │
│ ├── NodeJSExecutor              │
│ └── ProcessPool (Warm Starts)   │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Storage Layer (SQLite)          │
│ ├── Functions                   │
│ ├── Invocations                 │
│ └── Metrics                     │
└─────────────────────────────────┘
    ↓
[Monitoring Stack]
├── Prometheus (Metrics)
├── Grafana (Dashboards)
└── Loki (Logs)
```

---

## Prerequisites

### Required Software

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build dependencies
sudo apt install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    sqlite3 \
    nginx \
    certbot \
    python3-certbot-nginx

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Python (for Python runtime)
sudo apt install -y python3 python3-pip python3-venv

# Install Node.js (for Node.js runtime)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

### Optional (Recommended)

```bash
# Install monitoring tools
sudo apt install -y prometheus prometheus-node-exporter grafana

# Install fail2ban for security
sudo apt install -y fail2ban

# Install UFW firewall
sudo apt install -y ufw
```

---

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| CPU | 2 cores |
| RAM | 4 GB |
| Disk | 20 GB SSD |
| OS | Ubuntu 22.04+ LTS |
| Network | 100 Mbps |

### Recommended for Production

| Component | Requirement |
|-----------|-------------|
| CPU | 4+ cores |
| RAM | 8+ GB |
| Disk | 50+ GB SSD |
| OS | Ubuntu 24.04 LTS |
| Network | 1 Gbps |

### Scaling Guidelines

- **Small**: <100 req/s → 2 cores, 4 GB RAM
- **Medium**: 100-1000 req/s → 4 cores, 8 GB RAM
- **Large**: 1000-10000 req/s → 8+ cores, 16+ GB RAM
- **Enterprise**: >10000 req/s → Multi-node cluster

---

## Installation

### 1. Create Deployment User

```bash
# Create dedicated user for Nanolambda
sudo useradd -r -s /bin/bash -d /opt/nanolambda -m nanolambda

# Add user to necessary groups
sudo usermod -aG sudo nanolambda  # Only if needed for maintenance
```

### 2. Clone Repository

```bash
# Switch to nanolambda user
sudo -u nanolambda -i

# Clone repository
cd /opt/nanolambda
git clone https://github.com/ip888/nanolambda.git
cd nanolambda
```

### 3. Build Release Binary

```bash
# Build optimized release binary
cargo build --release --bin server

# Verify build
./target/release/server --version
```

### 4. Install Binary

```bash
# Copy binary to system location
sudo cp target/release/server /usr/local/bin/nanolambda-server
sudo chmod +x /usr/local/bin/nanolambda-server

# Verify installation
nanolambda-server --version
```

### 5. Create Directory Structure

```bash
# Create necessary directories
sudo mkdir -p /var/lib/nanolambda/{data,logs,functions}
sudo mkdir -p /etc/nanolambda

# Set ownership
sudo chown -R nanolambda:nanolambda /var/lib/nanolambda
sudo chown -R nanolambda:nanolambda /etc/nanolambda

# Set permissions
sudo chmod 755 /var/lib/nanolambda
sudo chmod 700 /var/lib/nanolambda/data
```

---

## systemd Service Configuration

### Create Service File

```bash
sudo nano /etc/systemd/system/nanolambda.service
```

**Contents**:

```ini
[Unit]
Description=Nanolambda Serverless Platform
Documentation=https://github.com/ip888/nanolambda
After=network.target

[Service]
Type=simple
User=nanolambda
Group=nanolambda
WorkingDirectory=/var/lib/nanolambda

# Environment variables
Environment="RUST_LOG=info"
Environment="NANOLAMBDA_HOST=127.0.0.1"
Environment="NANOLAMBDA_PORT=3000"
Environment="NANOLAMBDA_DB_PATH=/var/lib/nanolambda/data/nanolambda.db"
Environment="NANOLAMBDA_LOG_PATH=/var/lib/nanolambda/logs"

# Execution
ExecStart=/usr/local/bin/nanolambda-server
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/nanolambda

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nanolambda

[Install]
WantedBy=multi-user.target
```

### Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable nanolambda

# Start service
sudo systemctl start nanolambda

# Check status
sudo systemctl status nanolambda

# View logs
sudo journalctl -u nanolambda -f
```

### Service Management Commands

```bash
# Start service
sudo systemctl start nanolambda

# Stop service
sudo systemctl stop nanolambda

# Restart service
sudo systemctl restart nanolambda

# Reload configuration (without restart)
sudo systemctl reload nanolambda

# Check status
sudo systemctl status nanolambda

# View logs (last 100 lines)
sudo journalctl -u nanolambda -n 100

# Follow logs in real-time
sudo journalctl -u nanolambda -f

# View logs since specific time
sudo journalctl -u nanolambda --since "1 hour ago"
```

---

## Nginx Reverse Proxy

### Basic Configuration

```bash
sudo nano /etc/nginx/sites-available/nanolambda
```

**Contents**:

```nginx
# Upstream definition
upstream nanolambda {
    # Single server
    server 127.0.0.1:3000 fail_timeout=5s max_fails=3;
    
    # For multiple instances (load balancing)
    # server 127.0.0.1:3000 weight=1;
    # server 127.0.0.1:3001 weight=1;
    # server 127.0.0.1:3002 weight=1;
    
    keepalive 32;
}

# HTTP server (redirect to HTTPS)
server {
    listen 80;
    listen [::]:80;
    server_name nanolambda.yourdomain.com;
    
    # Let's Encrypt verification
    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }
    
    # Redirect all HTTP to HTTPS
    location / {
        return 301 https://$server_name$request_uri;
    }
}

# HTTPS server
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name nanolambda.yourdomain.com;
    
    # SSL certificates (will be configured with certbot)
    ssl_certificate /etc/letsencrypt/live/nanolambda.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/nanolambda.yourdomain.com/privkey.pem;
    
    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    
    # Logging
    access_log /var/log/nginx/nanolambda-access.log;
    error_log /var/log/nginx/nanolambda-error.log;
    
    # Client body size (for large function uploads)
    client_max_body_size 10M;
    
    # Timeouts
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;
    
    # Proxy settings
    location / {
        proxy_pass http://nanolambda;
        proxy_http_version 1.1;
        
        # Headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection "";
        
        # Buffering (disable for streaming)
        proxy_buffering off;
        
        # Keepalive
        proxy_socket_keepalive on;
    }
    
    # Health check endpoint (no auth required)
    location /health {
        proxy_pass http://nanolambda/health;
        access_log off;
    }
    
    # Metrics endpoint (restrict access)
    location /metrics {
        proxy_pass http://nanolambda/metrics;
        allow 127.0.0.1;
        allow 10.0.0.0/8;  # Adjust for your network
        deny all;
    }
}
```

### Enable Configuration

```bash
# Test configuration
sudo nginx -t

# Create symbolic link
sudo ln -s /etc/nginx/sites-available/nanolambda /etc/nginx/sites-enabled/

# Reload nginx
sudo systemctl reload nginx

# Check status
sudo systemctl status nginx
```

### Rate Limiting (Optional)

Add to nginx config for DDoS protection:

```nginx
# Define rate limit zones (add to http block)
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=100r/s;
limit_req_zone $binary_remote_addr zone=function_limit:10m rate=50r/s;

# Apply to location blocks
location /api/ {
    limit_req zone=api_limit burst=20 nodelay;
    proxy_pass http://nanolambda;
}

location /functions/ {
    limit_req zone=function_limit burst=10 nodelay;
    proxy_pass http://nanolambda;
}
```

---

## TLS/SSL Configuration

### Using Let's Encrypt (Recommended)

```bash
# Obtain certificate
sudo certbot --nginx -d nanolambda.yourdomain.com

# Test automatic renewal
sudo certbot renew --dry-run

# Certificate auto-renewal is configured by default
# Check with:
sudo systemctl status certbot.timer
```

### Using Self-Signed Certificate (Development)

```bash
# Generate self-signed certificate
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/ssl/private/nanolambda-selfsigned.key \
  -out /etc/ssl/certs/nanolambda-selfsigned.crt

# Update nginx to use self-signed cert
ssl_certificate /etc/ssl/certs/nanolambda-selfsigned.crt;
ssl_certificate_key /etc/ssl/private/nanolambda-selfsigned.key;
```

### Certificate Renewal Automation

```bash
# Create renewal hook
sudo nano /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh
```

```bash
#!/bin/bash
systemctl reload nginx
```

```bash
# Make executable
sudo chmod +x /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh
```

---

## Database Setup

### SQLite Configuration

```bash
# Create database directory
sudo mkdir -p /var/lib/nanolambda/data
sudo chown nanolambda:nanolambda /var/lib/nanolambda/data
sudo chmod 700 /var/lib/nanolambda/data

# Set database path in environment
# (already configured in systemd service)
NANOLAMBDA_DB_PATH=/var/lib/nanolambda/data/nanolambda.db
```

### Initialize Database

The database is automatically initialized on first run. To manually initialize:

```bash
# Run as nanolambda user
sudo -u nanolambda sqlite3 /var/lib/nanolambda/data/nanolambda.db < schema.sql
```

### Database Permissions

```bash
# Ensure proper permissions
sudo chown nanolambda:nanolambda /var/lib/nanolambda/data/nanolambda.db
sudo chmod 600 /var/lib/nanolambda/data/nanolambda.db
```

### Database Optimization

```sql
-- Run periodically for optimization
PRAGMA optimize;
PRAGMA vacuum;
PRAGMA analyze;
```

---

## Monitoring Stack

### Prometheus Setup

#### Install Prometheus

```bash
# Already installed via apt
sudo systemctl enable prometheus
sudo systemctl start prometheus
```

#### Configure Prometheus

```bash
sudo nano /etc/prometheus/prometheus.yml
```

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Nanolambda metrics
  - job_name: 'nanolambda'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: /metrics
    
  # Node exporter (system metrics)
  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
      
  # Prometheus itself
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
```

```bash
# Reload configuration
sudo systemctl reload prometheus

# Check targets
# Visit: http://localhost:9090/targets
```

### Grafana Setup

#### Install and Configure

```bash
# Enable and start Grafana
sudo systemctl enable grafana-server
sudo systemctl start grafana-server

# Default credentials: admin/admin
# Visit: http://localhost:3000
```

#### Add Prometheus Data Source

1. Login to Grafana (http://localhost:3000)
2. Go to Configuration → Data Sources
3. Click "Add data source"
4. Select "Prometheus"
5. Set URL to `http://localhost:9090`
6. Click "Save & Test"

#### Import Nanolambda Dashboard

Create dashboard with these key metrics:

**File**: `/etc/grafana/dashboards/nanolambda.json`

```json
{
  "dashboard": {
    "title": "Nanolambda Metrics",
    "panels": [
      {
        "title": "Requests per Second",
        "targets": [{
          "expr": "rate(http_requests_total[5m])"
        }]
      },
      {
        "title": "Response Time (p95)",
        "targets": [{
          "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(http_requests_total{status=~\"5..\"}[5m])"
        }]
      },
      {
        "title": "Function Invocations",
        "targets": [{
          "expr": "rate(function_invocations_total[5m])"
        }]
      },
      {
        "title": "Cold Starts",
        "targets": [{
          "expr": "rate(function_cold_starts_total[5m])"
        }]
      },
      {
        "title": "Memory Usage",
        "targets": [{
          "expr": "process_resident_memory_bytes"
        }]
      }
    ]
  }
}
```

### Alerting Rules

```bash
sudo nano /etc/prometheus/alert_rules.yml
```

```yaml
groups:
  - name: nanolambda_alerts
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} requests/sec"
      
      - alert: HighResponseTime
        expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        annotations:
          summary: "High response time"
          description: "P95 response time is {{ $value }}s"
      
      - alert: ServiceDown
        expr: up{job="nanolambda"} == 0
        for: 1m
        annotations:
          summary: "Nanolambda service is down"
          description: "Service has been down for more than 1 minute"
```

### Log Aggregation with Loki (Optional)

```bash
# Install Loki
wget https://github.com/grafana/loki/releases/download/v2.9.0/loki-linux-amd64.zip
unzip loki-linux-amd64.zip
sudo mv loki-linux-amd64 /usr/local/bin/loki

# Create config
sudo nano /etc/loki/config.yml
```

---

## Security Hardening

### Firewall Configuration

```bash
# Enable UFW
sudo ufw enable

# Allow SSH (be careful!)
sudo ufw allow 22/tcp

# Allow HTTP/HTTPS
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Deny direct access to Nanolambda port
sudo ufw deny 3000/tcp

# Allow Prometheus (only from localhost)
sudo ufw allow from 127.0.0.1 to any port 9090

# Check status
sudo ufw status verbose
```

### fail2ban Configuration

```bash
# Create jail for Nanolambda
sudo nano /etc/fail2ban/jail.d/nanolambda.conf
```

```ini
[nanolambda]
enabled = true
port = 80,443
filter = nanolambda
logpath = /var/log/nginx/nanolambda-access.log
maxretry = 10
findtime = 600
bantime = 3600
```

```bash
# Create filter
sudo nano /etc/fail2ban/filter.d/nanolambda.conf
```

```ini
[Definition]
failregex = ^<HOST> .* "(GET|POST|PUT|DELETE) .* HTTP/.*" (4|5)\d\d
ignoreregex =
```

```bash
# Restart fail2ban
sudo systemctl restart fail2ban

# Check status
sudo fail2ban-client status nanolambda
```

### API Authentication (Recommended)

Add API key authentication to your Nanolambda configuration:

```rust
// Example middleware (to be implemented in Task 7)
async fn auth_middleware(req: Request, next: Next) -> Result<Response> {
    let api_key = req.headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok());
    
    if api_key != Some(env::var("NANOLAMBDA_API_KEY")?) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(req).await)
}
```

### Secure Environment Variables

```bash
# Store sensitive data in a protected file
sudo nano /etc/nanolambda/secrets.env
```

```bash
NANOLAMBDA_API_KEY=your-secure-random-key-here
NANOLAMBDA_DB_ENCRYPTION_KEY=another-secure-key
```

```bash
# Protect the file
sudo chmod 600 /etc/nanolambda/secrets.env
sudo chown nanolambda:nanolambda /etc/nanolambda/secrets.env

# Update systemd service
sudo nano /etc/systemd/system/nanolambda.service
```

Add:
```ini
EnvironmentFile=/etc/nanolambda/secrets.env
```

### Regular Security Updates

```bash
# Create update script
sudo nano /usr/local/bin/nanolambda-update.sh
```

```bash
#!/bin/bash
set -e

echo "Updating Nanolambda..."

# Update system
sudo apt update && sudo apt upgrade -y

# Backup database
sudo -u nanolambda cp /var/lib/nanolambda/data/nanolambda.db \
  /var/lib/nanolambda/data/nanolambda.db.backup.$(date +%Y%m%d_%H%M%S)

# Pull latest code
cd /opt/nanolambda/nanolambda
sudo -u nanolambda git pull

# Build new binary
sudo -u nanolambda cargo build --release --bin server

# Replace binary
sudo cp target/release/server /usr/local/bin/nanolambda-server

# Restart service
sudo systemctl restart nanolambda

echo "Update complete!"
```

```bash
sudo chmod +x /usr/local/bin/nanolambda-update.sh
```

---

## Backup & Recovery

### Automated Backup Script

```bash
sudo nano /usr/local/bin/nanolambda-backup.sh
```

```bash
#!/bin/bash
set -e

BACKUP_DIR="/var/backups/nanolambda"
DATE=$(date +%Y%m%d_%H%M%S)
DB_PATH="/var/lib/nanolambda/data/nanolambda.db"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup database
echo "Backing up database..."
sudo -u nanolambda sqlite3 "$DB_PATH" ".backup '$BACKUP_DIR/nanolambda_$DATE.db'"

# Compress backup
gzip "$BACKUP_DIR/nanolambda_$DATE.db"

# Remove backups older than 30 days
find "$BACKUP_DIR" -name "*.db.gz" -mtime +30 -delete

echo "Backup complete: $BACKUP_DIR/nanolambda_$DATE.db.gz"
```

```bash
sudo chmod +x /usr/local/bin/nanolambda-backup.sh
```

### Schedule Backups with Cron

```bash
# Edit crontab for root
sudo crontab -e
```

Add:
```cron
# Backup Nanolambda database daily at 2 AM
0 2 * * * /usr/local/bin/nanolambda-backup.sh >> /var/log/nanolambda-backup.log 2>&1
```

### Restore from Backup

```bash
#!/bin/bash
# Restore script

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.db.gz>"
    exit 1
fi

# Stop service
sudo systemctl stop nanolambda

# Decompress backup
gunzip -c "$BACKUP_FILE" > /tmp/nanolambda_restore.db

# Replace database
sudo cp /tmp/nanolambda_restore.db /var/lib/nanolambda/data/nanolambda.db
sudo chown nanolambda:nanolambda /var/lib/nanolambda/data/nanolambda.db

# Start service
sudo systemctl start nanolambda

echo "Restore complete!"
```

### Off-Site Backup (Recommended)

```bash
# Sync to S3 (requires aws-cli)
aws s3 sync /var/backups/nanolambda s3://your-bucket/nanolambda-backups/

# Or use rsync to remote server
rsync -avz /var/backups/nanolambda/ backup-server:/backups/nanolambda/
```

---

## Scaling Strategies

### Vertical Scaling (Single Server)

**Increase Resources**:
- Add more CPU cores
- Increase RAM
- Use faster SSD storage
- Optimize kernel parameters

```bash
# Increase file descriptor limits
sudo nano /etc/security/limits.conf
```

Add:
```
nanolambda soft nofile 65536
nanolambda hard nofile 65536
```

```bash
# Optimize kernel parameters
sudo nano /etc/sysctl.conf
```

Add:
```
# Network optimizations
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.ip_local_port_range = 1024 65535

# File system
fs.file-max = 2097152
```

Apply:
```bash
sudo sysctl -p
```

### Horizontal Scaling (Multiple Servers)

#### Load Balancer Configuration

Update nginx upstream:

```nginx
upstream nanolambda {
    least_conn;  # or ip_hash for sticky sessions
    
    server 10.0.1.10:3000 weight=1 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:3000 weight=1 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:3000 weight=1 max_fails=3 fail_timeout=30s;
    
    keepalive 64;
}
```

#### Shared Database Approach

For multi-node setup, consider:

1. **Shared NFS Mount** (Simple)
   ```bash
   # Mount shared storage for database
   sudo mount -t nfs nfs-server:/nanolambda /var/lib/nanolambda/data
   ```

2. **PostgreSQL** (Advanced)
   - Migrate from SQLite to PostgreSQL
   - Better concurrency support
   - Replication capabilities

3. **Function Code Distribution**
   - Use shared storage (NFS/S3) for function code
   - Cache locally on each node

#### Health Check Endpoint

Ensure `/health` endpoint is implemented:

```rust
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now().to_rfc3339()
    }))
}
```

---

## Health Checks

### Service Health Check

```bash
# Create health check script
sudo nano /usr/local/bin/nanolambda-healthcheck.sh
```

```bash
#!/bin/bash

ENDPOINT="http://localhost:3000/health"
EXPECTED_STATUS=200

# Check HTTP status
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$ENDPOINT")

if [ "$STATUS" -eq "$EXPECTED_STATUS" ]; then
    echo "OK: Service is healthy (HTTP $STATUS)"
    exit 0
else
    echo "CRITICAL: Service returned HTTP $STATUS"
    exit 2
fi
```

```bash
sudo chmod +x /usr/local/bin/nanolambda-healthcheck.sh
```

### Automated Health Monitoring

```bash
# Add to cron for periodic checks
*/5 * * * * /usr/local/bin/nanolambda-healthcheck.sh || /usr/local/bin/alert-admin.sh
```

### External Monitoring

Use services like:
- **UptimeRobot** (Free tier available)
- **Pingdom**
- **StatusCake**
- **Custom monitoring with Prometheus Alertmanager**

---

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check service status
sudo systemctl status nanolambda

# View detailed logs
sudo journalctl -u nanolambda -n 100 --no-pager

# Check for port conflicts
sudo lsof -i :3000

# Verify binary exists
ls -la /usr/local/bin/nanolambda-server

# Test binary directly
sudo -u nanolambda /usr/local/bin/nanolambda-server
```

#### High Memory Usage

```bash
# Check process memory
ps aux | grep nanolambda

# Monitor in real-time
top -p $(pgrep -f nanolambda)

# Check for memory leaks
sudo systemctl restart nanolambda  # Temporary fix

# Review process pool settings
# Adjust max_pool_size in configuration
```

#### Database Locked

```bash
# Check for concurrent access
sudo lsof /var/lib/nanolambda/data/nanolambda.db

# Verify permissions
ls -la /var/lib/nanolambda/data/nanolambda.db

# Check database integrity
sudo -u nanolambda sqlite3 /var/lib/nanolambda/data/nanolambda.db "PRAGMA integrity_check;"
```

#### Nginx Errors

```bash
# Test configuration
sudo nginx -t

# Check error logs
sudo tail -f /var/log/nginx/error.log

# Verify upstream connection
curl http://127.0.0.1:3000/health
```

### Debug Mode

Enable debug logging:

```bash
# Edit systemd service
sudo nano /etc/systemd/system/nanolambda.service
```

Change:
```ini
Environment="RUST_LOG=debug"
```

```bash
# Reload and restart
sudo systemctl daemon-reload
sudo systemctl restart nanolambda

# View debug logs
sudo journalctl -u nanolambda -f
```

### Performance Issues

```bash
# Check system resources
htop

# Disk I/O
iostat -x 1

# Network connections
ss -tunapl | grep 3000

# Database performance
sudo -u nanolambda sqlite3 /var/lib/nanolambda/data/nanolambda.db ".timer on" "SELECT COUNT(*) FROM functions;"
```

---

## Cloud Provider Deployments

### AWS (Amazon Web Services)

#### Option 1: EC2 Instance

**Instance Selection**:
- **Small**: t3.small (2 vCPU, 2GB RAM) - $15/month
- **Medium**: t3.medium (2 vCPU, 4GB RAM) - $30/month
- **Large**: c5.xlarge (4 vCPU, 8GB RAM) - $145/month

**Quick Setup**:

```bash
# 1. Launch EC2 instance
# - AMI: Ubuntu 24.04 LTS
# - Instance type: t3.medium
# - Storage: 20GB gp3
# - Security group: Allow 22 (SSH), 80 (HTTP), 443 (HTTPS)

# 2. Connect to instance
ssh -i your-key.pem ubuntu@your-ec2-public-ip

# 3. Follow standard installation from this guide
# (Prerequisites → Installation → Configuration)

# 4. Configure security group
# AWS Console → EC2 → Security Groups → Add rules:
# - Type: SSH, Port: 22, Source: Your IP
# - Type: HTTP, Port: 80, Source: 0.0.0.0/0
# - Type: HTTPS, Port: 443, Source: 0.0.0.0/0

# 5. Attach Elastic IP (optional, for stable IP)
# AWS Console → EC2 → Elastic IPs → Allocate → Associate
```

**Storage Options**:

```bash
# Use EBS for database (recommended)
# Already included with EC2 instance

# Or use EFS for shared storage across instances
sudo apt install -y nfs-common
sudo mount -t nfs4 -o nfsvers=4.1 fs-xxxxx.efs.us-east-1.amazonaws.com:/ /var/lib/nanolambda/data
```

**Load Balancer Setup**:

```bash
# AWS Console → EC2 → Load Balancers → Create Application Load Balancer
# 1. Configure:
#    - Name: nanolambda-alb
#    - Scheme: Internet-facing
#    - IP address type: IPv4
#    - Listeners: HTTP (80), HTTPS (443)
#
# 2. Availability Zones: Select 2+ subnets
#
# 3. Security Groups: Allow 80, 443
#
# 4. Target Group:
#    - Name: nanolambda-targets
#    - Protocol: HTTP
#    - Port: 3000
#    - Health check: /health
#
# 5. Register EC2 instances
```

**Auto Scaling**:

```bash
# AWS Console → EC2 → Auto Scaling Groups
# 1. Create Launch Template from your configured instance
# 2. Create Auto Scaling Group:
#    - Desired: 2
#    - Minimum: 1
#    - Maximum: 10
# 3. Scaling Policies:
#    - Target tracking: CPU 70%
#    - Target tracking: ALB requests 1000/target
```

**RDS for Database (Advanced)**:

```bash
# For high-availability, migrate to PostgreSQL on RDS
# AWS Console → RDS → Create Database
# - Engine: PostgreSQL 15
# - Template: Production
# - Instance: db.t3.medium
# - Storage: 100GB gp3
# - Multi-AZ: Yes

# Update Nanolambda to use PostgreSQL (requires code changes in Task 7)
```

**S3 for Backups**:

```bash
# Create S3 bucket
aws s3 mb s3://nanolambda-backups-yourcompany

# Update backup script
sudo tee -a /usr/local/bin/nanolambda-backup.sh > /dev/null <<'EOF'
# Sync to S3
aws s3 sync /var/backups/nanolambda s3://nanolambda-backups-yourcompany/ \
  --storage-class STANDARD_IA
EOF
```

**Cost Optimization**:
- Use Reserved Instances (save 40-60%)
- Use Spot Instances for dev/test (save up to 90%)
- Enable detailed CloudWatch monitoring
- Set up cost alerts

---

### Digital Ocean

#### Droplet Deployment

**Droplet Selection**:
- **Small**: Basic 2GB - $12/month
- **Medium**: Basic 4GB - $24/month
- **Large**: General Purpose 8GB - $48/month

**Quick Setup**:

```bash
# 1. Create Droplet
# - Distribution: Ubuntu 24.04 LTS
# - Plan: Basic 4GB ($24/mo)
# - Datacenter: Choose closest to users
# - Additional: Monitoring (free), Backups (+20%)

# 2. Add SSH key during creation

# 3. Connect
ssh root@your-droplet-ip

# 4. Initial setup
adduser nanolambda
usermod -aG sudo nanolambda
ufw allow OpenSSH
ufw enable

# 5. Follow standard installation from this guide
```

**Managed Database**:

```bash
# Digital Ocean → Databases → Create Database Cluster
# - Engine: PostgreSQL 15
# - Plan: Basic 1GB - $15/month
# - Datacenter: Same as droplet

# Connection string:
postgresql://user:pass@host:25060/nanolambda?sslmode=require

# Update Nanolambda config (Task 7 will add PostgreSQL support)
```

**Load Balancer**:

```bash
# Digital Ocean → Networking → Load Balancers → Create
# - Name: nanolambda-lb
# - Type: Regional
# - Forwarding Rules:
#   - HTTPS 443 → HTTP 3000
#   - HTTP 80 → HTTP 3000 (redirect to HTTPS)
# - Health Check: /health
# - Sticky Sessions: Enabled
# - Add Droplets

# Update DNS to point to load balancer IP
```

**Spaces for Backups** (S3-compatible):

```bash
# Create Space
# Digital Ocean → Spaces → Create Space
# - Name: nanolambda-backups
# - Region: Same as droplet

# Install s3cmd
sudo apt install -y s3cmd

# Configure s3cmd
s3cmd --configure
# Access Key: From API → Spaces Keys
# Secret Key: From API → Spaces Keys
# S3 Endpoint: nyc3.digitaloceanspaces.com

# Update backup script
sudo tee -a /usr/local/bin/nanolambda-backup.sh > /dev/null <<'EOF'
# Sync to Spaces
s3cmd sync /var/backups/nanolambda/ s3://nanolambda-backups/
EOF
```

**Floating IP** (Static IP):

```bash
# Digital Ocean → Networking → Floating IPs → Create
# Assign to droplet

# Update DNS A record to floating IP
# Benefits: Can reassign to different droplet without DNS changes
```

**Monitoring**:

```bash
# Enable Droplet monitoring (free)
# Includes: CPU, memory, disk, bandwidth

# Install Digital Ocean monitoring agent
curl -sSL https://repos.insights.digitalocean.com/install.sh | sudo bash

# View metrics in Digital Ocean Dashboard
```

**Cost Optimization**:
- Enable weekly backups (+20%, worth it)
- Use shared CPU droplets for non-production
- Resize droplets during off-peak hours
- Use CDN for static assets

---

### Google Cloud Platform (GCP)

#### Compute Engine VM

**Instance Selection**:
- **Small**: e2-small (2 vCPU, 2GB) - $13/month
- **Medium**: e2-medium (2 vCPU, 4GB) - $27/month
- **Large**: n2-standard-2 (2 vCPU, 8GB) - $73/month

**Quick Setup**:

```bash
# 1. Create VM Instance
# GCP Console → Compute Engine → VM Instances → Create
# - Name: nanolambda-1
# - Region: us-central1
# - Machine type: e2-medium
# - Boot disk: Ubuntu 24.04 LTS, 20GB SSD
# - Firewall: Allow HTTP, HTTPS

# 2. Connect via SSH (browser or gcloud)
gcloud compute ssh nanolambda-1

# 3. Follow standard installation
```

**Cloud SQL**:

```bash
# GCP Console → SQL → Create Instance
# - Database engine: PostgreSQL 15
# - Instance ID: nanolambda-db
# - Region: Same as VM
# - Machine type: db-f1-micro ($7.67/month)
# - Storage: 10GB SSD

# Connect from VM using Cloud SQL Proxy
curl -o cloud-sql-proxy https://dl.google.com/cloudsql/cloud_sql_proxy.linux.amd64
chmod +x cloud-sql-proxy
./cloud-sql-proxy --instances=PROJECT:REGION:INSTANCE=tcp:5432
```

**Load Balancing**:

```bash
# GCP Console → Network Services → Load Balancing
# 1. Create HTTP(S) Load Balancer
# 2. Backend configuration:
#    - Instance group: Create with your VMs
#    - Health check: /health
# 3. Frontend configuration:
#    - Protocol: HTTPS
#    - IP: Create static IP
# 4. SSL certificate: Create managed certificate
```

**Cloud Storage for Backups**:

```bash
# Create bucket
gsutil mb -c STANDARD -l us-central1 gs://nanolambda-backups/

# Update backup script
sudo tee -a /usr/local/bin/nanolambda-backup.sh > /dev/null <<'EOF'
# Sync to Cloud Storage
gsutil -m rsync -r /var/backups/nanolambda gs://nanolambda-backups/
EOF

# Set lifecycle policy (delete after 90 days)
gsutil lifecycle set lifecycle.json gs://nanolambda-backups/
```

---

### Linode (Akamai)

#### Linode Instance

**Plan Selection**:
- **Small**: Nanode 1GB - $5/month
- **Medium**: Linode 4GB - $24/month
- **Large**: Dedicated 8GB - $96/month

**Quick Setup**:

```bash
# 1. Create Linode
# Cloud Manager → Linodes → Create
# - Distribution: Ubuntu 24.04 LTS
# - Region: Choose closest
# - Plan: Linode 4GB
# - Add SSH key

# 2. Connect
ssh root@linode-ip

# 3. Follow standard installation
```

**NodeBalancer**:

```bash
# Cloud Manager → NodeBalancers → Create
# - Region: Same as Linodes
# - Configuration:
#   - Port: 443 (HTTPS)
#   - Protocol: HTTP
#   - Algorithm: Round Robin
# - Health Check: /health
# - Add Linodes as backends
```

**Block Storage**:

```bash
# For shared database across instances
# Cloud Manager → Volumes → Create
# - Label: nanolambda-data
# - Size: 100GB
# - Region: Same as Linode

# Mount on Linode
mkdir -p /mnt/nanolambda-data
mount /dev/disk/by-id/scsi-0Linode_Volume_nanolambda-data /mnt/nanolambda-data

# Update database path
NANOLAMBDA_DB_PATH=/mnt/nanolambda-data/nanolambda.db
```

**Object Storage for Backups**:

```bash
# Cloud Manager → Object Storage → Create Bucket
# - Label: nanolambda-backups
# - Region: us-east-1

# Configure s3cmd (Linode is S3-compatible)
# Endpoint: us-east-1.linodeobjects.com
```

---

### Hetzner Cloud

#### Server Deployment

**Server Selection** (Best price/performance):
- **Small**: CX21 (2 vCPU, 4GB) - €5.83/month (~$6.30)
- **Medium**: CX31 (2 vCPU, 8GB) - €10.69/month (~$11.60)
- **Large**: CX41 (4 vCPU, 16GB) - €20.33/month (~$22)

**Quick Setup**:

```bash
# 1. Create Server
# Hetzner Cloud Console → Servers → Add Server
# - Location: Nuremberg (Germany) or Ashburn (US)
# - Image: Ubuntu 24.04
# - Type: CX31
# - SSH Key: Add your key

# 2. Connect
ssh root@server-ip

# 3. Follow standard installation
```

**Load Balancer**:

```bash
# Hetzner Cloud Console → Load Balancers → Create
# - Name: nanolambda-lb
# - Type: LB11 (€5.83/month)
# - Location: Same as servers
# - Services:
#   - HTTPS 443 → HTTP 3000
#   - Health check: /health
# - Add servers as targets
```

**Volume for Database**:

```bash
# Create Volume
# Console → Volumes → Add Volume
# - Size: 100GB (€4/month)
# - Format: ext4
# - Auto-mount: Yes

# Database will be on /mnt/HC_Volume_xxxxx
```

---

### Vultr

#### Instance Deployment

**Plan Selection**:
- **Small**: Regular Performance 2GB - $12/month
- **Medium**: Regular Performance 4GB - $24/month
- **Large**: High Performance 8GB - $48/month

**Quick Setup**:

```bash
# 1. Deploy Instance
# Vultr → Deploy → Cloud Compute
# - Location: Choose closest
# - Server Type: Cloud Compute - Regular Performance
# - Plan: 4GB RAM
# - OS: Ubuntu 24.04 x64

# 2. Connect
ssh root@vultr-ip

# 3. Follow standard installation
```

---

### General Cloud Best Practices

#### Security Groups / Firewall Rules

All cloud providers - configure these firewall rules:

```
Inbound:
- SSH (22): Your IP only
- HTTP (80): 0.0.0.0/0 (redirect to HTTPS)
- HTTPS (443): 0.0.0.0/0
- Prometheus (9090): Internal only
- Grafana (3000): Your IP only (or use SSH tunnel)

Outbound:
- All traffic: 0.0.0.0/0 (for updates, packages)
```

#### Monitoring Integration

```bash
# All clouds support Prometheus exporters
# Install node_exporter on each instance
wget https://github.com/prometheus/node_exporter/releases/download/v1.7.0/node_exporter-1.7.0.linux-amd64.tar.gz
tar xvf node_exporter-1.7.0.linux-amd64.tar.gz
sudo cp node_exporter-1.7.0.linux-amd64/node_exporter /usr/local/bin/
sudo useradd -rs /bin/false node_exporter

# Create systemd service
sudo tee /etc/systemd/system/node_exporter.service > /dev/null <<'EOF'
[Unit]
Description=Node Exporter

[Service]
User=node_exporter
ExecStart=/usr/local/bin/node_exporter

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now node_exporter
```

#### Backup Strategy

```bash
# Use cloud provider's snapshot feature + off-site backups

# AWS: EBS Snapshots
aws ec2 create-snapshot --volume-id vol-xxxxx --description "Daily backup"

# Digital Ocean: Droplet Backups (automated)
# Enable during droplet creation

# GCP: Persistent Disk Snapshots
gcloud compute disks snapshot DISK_NAME --snapshot-names=backup-$(date +%Y%m%d)

# Hetzner: Automated Backups
# Enable in server settings (20% of server cost)
```

#### Cost Comparison Summary

| Provider | 4GB Instance | Load Balancer | Database | Storage | Total/Month |
|----------|--------------|---------------|----------|---------|-------------|
| AWS EC2 | $30 | $16 | $30 (RDS) | $10 (EBS) | ~$86 |
| Digital Ocean | $24 | $12 | $15 (Managed) | $5 (Spaces) | ~$56 |
| GCP | $27 | $18 | $8 (SQL) | $2 (Cloud Storage) | ~$55 |
| Linode | $24 | $10 | - | $5 (Object) | ~$39 |
| Hetzner | $12 | $6 | - | $4 (Volume) | ~$22 |
| Vultr | $24 | - | - | $5 | ~$29 |

**Best Value**: Hetzner Cloud (Europe) or Linode (Global)  
**Best Ecosystem**: AWS (if already using AWS services)  
**Best Simplicity**: Digital Ocean (excellent UX)

---

## Troubleshooting

### Application Tuning

#### Process Pool Configuration

Adjust in your configuration:

```toml
# /etc/nanolambda/config.toml
[runtime]
max_pool_size = 100          # Max processes per function
max_age_seconds = 300        # Process lifetime (5 min)
enable_warm_starts = true

[server]
worker_threads = 4           # Match CPU cores
max_connections = 1000
```

#### Database Optimization

```sql
-- Run during maintenance window
PRAGMA journal_mode = WAL;       -- Write-Ahead Logging
PRAGMA synchronous = NORMAL;     -- Balance safety/performance
PRAGMA cache_size = -64000;      -- 64MB cache
PRAGMA temp_store = MEMORY;      -- Use RAM for temp tables
```

### System Tuning

#### TCP Optimization

```bash
# /etc/sysctl.conf
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_max_syn_backlog = 8192
net.core.netdev_max_backlog = 5000
```

#### File System

```bash
# Use faster filesystem
# ext4 with noatime option
/dev/sda1 /var/lib/nanolambda ext4 defaults,noatime 0 2
```

### Monitoring Performance

```bash
# Install performance monitoring tools
sudo apt install -y sysstat iotop iftop

# CPU usage by process
pidstat -p $(pgrep nanolambda) 1

# Disk I/O by process
sudo iotop -p $(pgrep nanolambda)

# Network usage
sudo iftop -i eth0
```

---

## Conclusion

This guide provides a comprehensive foundation for deploying Nanolambda in production. Key takeaways:

✅ **Security First**: Firewall, TLS, fail2ban, regular updates  
✅ **Monitoring**: Prometheus + Grafana for visibility  
✅ **Reliability**: systemd, health checks, automated backups  
✅ **Performance**: Tuned kernel parameters, optimized database  
✅ **Scalability**: Horizontal and vertical scaling strategies  

### Next Steps

1. Complete Task 7: StorageManager Integration
2. Implement API authentication
3. Add more runtime languages (Java, Go)
4. Develop CLI tool for management
5. Create Terraform/Ansible deployment automation

### Support

- **Documentation**: https://github.com/ip888/nanolambda/docs
- **Issues**: https://github.com/ip888/nanolambda/issues
- **Discussions**: https://github.com/ip888/nanolambda/discussions

---

**Version History**:
- v1.0 (2024-10-18): Initial release

**License**: MIT
