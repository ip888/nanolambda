#!/bin/bash
# Automated Docs Folder Cleanup Script
# Organizes documentation into proper structure

set -e

cd /workspaces/nanolambda/docs

echo "🧹 Docs Folder Cleanup"
echo "====================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_status() { echo -e "${GREEN}✓${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }

echo "This will organize 42 files:"
echo "  • Delete 6 duplicate files"
echo "  • Move 15 completion reports to archive/"
echo "  • Move 6 business docs to archive/business/"
echo "  • Move 8 implementation plans to archive/plans/ (keep 3 microVM docs)"
echo "  • Move 7 status reports to status/"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

echo ""
echo "🚀 Starting cleanup..."
echo ""

# =====================================
# Priority 1: Delete Duplicates
# =====================================

echo "Priority 1: Removing duplicate files..."

DUPLICATES=(
    "15-referral-program.md"
    "16-annual-billing.md"
    "17-usage-analytics.md"
    "18-customer-lifetime-value.md"
    "19-churn-analysis-prevention.md"
    "20-payment-retry-logic.md"
)

for file in "${DUPLICATES[@]}"; do
    if [ -f "$file" ]; then
        rm "$file"
        print_status "Deleted duplicate: $file"
    fi
done

# =====================================
# Priority 2: Archive Completion Reports
# =====================================

echo ""
echo "Priority 2: Archiving completion reports..."

COMPLETION_REPORTS=(
    "BENCHMARK_SUITE_COMPLETE.md"
    "DEPENDENCY_UPDATE_COMPLETE.md"
    "MEMORY_TRACKING_COMPLETE.md"
    "MODERNIZATION_COMPLETE.md"
    "NODEJS_RUNTIME_COMPLETE.md"
    "PROJECT_COMPLETE.md"
    "SESSION_SUMMARY.md"
    "TODAY_SUMMARY.md"
    "UPDATE_SUMMARY.md"
    "TASK_3_SUMMARY.md"
    "TASK_4_SUMMARY.md"
    "TASK_6_CLOUD_UPDATE.md"
    "TASK_7_COMPLETION.md"
    "TASK_7_IMPLEMENTATION_SUMMARY.md"
    "TASK_7_INTEGRATION_PLAN.md"
)

for file in "${COMPLETION_REPORTS[@]}"; do
    if [ -f "$file" ]; then
        mv "$file" archive/
        print_status "Archived: $file"
    fi
done

# =====================================
# Priority 3: Archive Business Docs
# =====================================

echo ""
echo "Priority 3: Archiving business planning docs..."

mkdir -p archive/business

BUSINESS_DOCS=(
    "00-executive-summary.md"
    "01-market-analysis.md"
    "02-technical-architecture.md"
    "04-roadmap.md"
    "04-roadmap-updated.md"
    "30_DAY_EXECUTION_PLAN.md"
)

for file in "${BUSINESS_DOCS[@]}"; do
    if [ -f "$file" ]; then
        mv "$file" archive/business/
        print_status "Archived business doc: $file"
    fi
done

# =====================================
# Priority 4: Archive Implementation Plans
# =====================================

echo ""
echo "Priority 4: Archiving implementation plans..."

mkdir -p archive/plans

IMPLEMENTATION_PLANS=(
    "nodejs-implementation-plan.md"
    "memory-tracking-plan.md"
    "warm-start-implementation.md"
    "SMART_WARM_START_TECHNICAL_GUIDE.md"
    "WARM_START_OPTIMIZATION_ANALYSIS.md"
    "IN_MEMORY_COLD_START_IMPLEMENTATION.md"
    "VMM_STATUS_AND_KVM.md"
    "DEPENDENCY_AUDIT.md"
)

# Keep microVM docs in docs/ (active development plans)
# These are NOT moved: MICROVM_ACTION_PLAN.md, MICROVM_IMPLEMENTATION_PLAN.md, microvm_recommendations.md

for file in "${IMPLEMENTATION_PLANS[@]}"; do
    if [ -f "$file" ]; then
        mv "$file" archive/plans/
        print_status "Archived plan: $file"
    fi
done

# =====================================
# Priority 5: Organize Status Reports
# =====================================

echo ""
echo "Priority 5: Organizing status reports..."

mkdir -p status

STATUS_REPORTS=(
    "PROJECT_PROGRESS.md"
    "PROJECT_AUDIT_REPORT.md"
    "BUGFIX_STORAGE_RECREATION.md"
    "PLATFORM_VERIFICATION.md"
    "PRODUCTION_USE_CASES.md"
    "STRATEGIC_ROADMAP.md"
    "HANDOFF.md"
)

for file in "${STATUS_REPORTS[@]}"; do
    if [ -f "$file" ]; then
        mv "$file" status/
        print_status "Moved to status: $file"
    fi
done

# =====================================
# Create Docs Index
# =====================================

echo ""
echo "Creating docs/README.md index..."

cat > README.md << 'EOF'
# NanoLambda Documentation

## 📚 Core Technical Documentation

### API & Authentication
- [API Authentication](API_AUTHENTICATION.md) - API key authentication with Bearer tokens
- [API Versioning](API_VERSIONING.md) - API versioning strategy
- [Feature Versioning](FEATURE_VERSIONING.md) - Feature versioning system

### Architecture & Design
- [Runtime Trait Design](runtime-trait-design.md) - Runtime abstraction layer
- [Storage Layer Design](storage-layer-design.md) - SQLite storage implementation
- [Dashboard Architecture](dashboard-architecture.md) - Dashboard design & metrics
- [Dashboard & Metrics](DASHBOARD_AND_METRICS.md) - Metrics collection system

### Deployment & Operations
- [Setup Guide](setup-guide.md) - Development environment setup
- [Deployment Quickstart](DEPLOYMENT_QUICKSTART.md) - Quick deployment guide
- [Production Deployment](PRODUCTION_DEPLOYMENT.md) - Production deployment guide
- [Observability](OBSERVABILITY.md) - Monitoring and logging
- [User Code Deployment](USER_CODE_DEPLOYMENT.md) - Deploy user functions

### Testing
- [Server Test Guide](SERVER_TEST_GUIDE.md) - Backend testing guide
- [Real World Testing Guide](REAL_WORLD_TESTING_GUIDE.md) - End-to-end testing

### Design Decisions
- [Versioning Decision](VERSIONING_DECISION.md) - Why we chose our versioning approach

---

## 🚀 Future Features

See [FUTURE_FEATURES/](FUTURE_FEATURES/) for planned enhancements:
- Customer portal
- Metered billing & Stripe integration
- Email notifications
- Usage alerts
- Discount codes & referral program
- Invoice system
- And more...

---

## 📊 Project Status

See [status/](status/) for current project status and progress reports.

---

## 🗄️ Archive

Historical documentation, completion reports, and implementation plans are in [archive/](archive/):
- [archive/](archive/) - Completion reports and session summaries
- [archive/business/](archive/business/) - Business planning documents
- [archive/plans/](archive/plans/) - Implementation plans and technical guides

---

## 🎯 Milestones

See [milestones/](milestones/) for major project milestones and achievements.
EOF

print_status "Created docs/README.md"

# =====================================
# Summary
# =====================================

echo ""
echo "=============================="
echo "✅ Cleanup Complete!"
echo "=============================="
echo ""
echo "Summary:"
echo "  • Deleted 6 duplicate files"
echo "  • Moved 15 files to archive/"
echo "  • Moved 6 files to archive/business/"
echo "  • Moved 8 files to archive/plans/"
echo "  • Moved 7 files to status/"
echo "  • Kept 3 microVM docs in root (active planning)"
echo "  • Created docs/README.md"
echo ""
echo "Docs structure:"
echo "  📁 docs/ (15 core technical docs)"
echo "  📁 docs/FUTURE_FEATURES/ (12 feature specs)"
echo "  📁 docs/archive/ (30+ historical docs)"
echo "  📁 docs/status/ (7 status reports)"
echo "  📁 docs/milestones/ (4 milestone docs)"
echo ""

# Count remaining files
REMAINING=$(ls -1 *.md 2>/dev/null | wc -l)
echo "Files in docs/ root: $REMAINING (down from 70)"
echo ""
print_status "Done! Documentation is now organized."
