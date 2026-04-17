# DigitalOcean + GitHub CI/CD Deployment

Complete automated deployment setup created! ✅

## Files Created:

1. **`.github/workflows/deploy-backend.yml`** - Auto-deploys backend on push
2. **`.github/workflows/deploy-frontend.yml`** - Auto-deploys frontend on push
3. **`Dockerfile`** - Production Docker image for backend
4. **`scripts/setup-digitalocean.sh`** - One-time droplet setup
5. **`scripts/deploy.sh`** - Manual deployment script

## Quick Start:

### 1. Create DigitalOcean Droplet
- Ubuntu 24.04 LTS
- 2GB RAM ($12/month)
- Note the IP address

### 2. Setup Droplet (one-time)
```bash
scp scripts/setup-digitalocean.sh root@YOUR_IP:/tmp/
ssh root@YOUR_IP "bash /tmp/setup-digitalocean.sh"
```

### 3. Configure GitHub Secrets
Go to: GitHub repo → Settings → Secrets → Add:
- `DO_HOST`: Your droplet IP
- `DO_USERNAME`: `root`
- `DO_SSH_KEY`: Your private SSH key
- `API_URL`: `https://api.nanolambda.com`

### 4. Update Domain in Caddyfile
```bash
ssh root@YOUR_IP
nano /etc/caddy/Caddyfile
# Replace nanolambda.com with your domain
systemctl reload caddy
```

### 5. Push to Deploy!
```bash
git push origin main
```

GitHub Actions automatically:
- ✅ Builds Docker image
- ✅ Deploys to DigitalOcean
- ✅ Restarts services
- ✅ Runs health checks

## Architecture:

```
GitHub Push → GitHub Actions → Docker Build → Deploy to Droplet

Client Request → Caddy (SSL) → Backend (8080) or Frontend (3000)
```

## Monitor:
```bash
# Backend logs
docker logs -f nanolambda-server

# Frontend logs
pm2 logs nanolambda-website

# Check status
curl https://api.nanolambda.com/health
```

See full guide: [docs/DEPLOYMENT_DIGITALOCEAN.md]
