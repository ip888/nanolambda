#!/bin/bash
# NanoLambda Project Cleanup Script
# Removes outdated and redundant documentation

set -e  # Exit on error

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║         NanoLambda Project Cleanup Script                      ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Navigate to project root
cd /workspaces/nanolambda

# Create backup branch (safety)
echo "📦 Creating safety backup..."
git branch -D cleanup-backup 2>/dev/null || true
git branch cleanup-backup
echo "✅ Backup branch 'cleanup-backup' created"
echo ""

# Phase 1: Delete Historical/Outdated Docs
echo "🗑️  Phase 1: Removing historical/outdated documentation..."

# Session summaries
rm -f docs/SESSION_SUMMARY.md docs/TODAY_SUMMARY.md
rm -f docs/TASK_3_SUMMARY.md docs/TASK_4_SUMMARY.md docs/TASK_6_CLOUD_UPDATE.md
rm -f docs/TASK_7_COMPLETION.md docs/TASK_7_IMPLEMENTATION_SUMMARY.md docs/TASK_7_INTEGRATION_PLAN.md

# Completion status files
rm -f docs/BENCHMARK_SUITE_COMPLETE.md docs/DEPENDENCY_UPDATE_COMPLETE.md
rm -f docs/MEMORY_TRACKING_COMPLETE.md docs/MODERNIZATION_COMPLETE.md
rm -f docs/NODEJS_RUNTIME_COMPLETE.md docs/PLATFORM_VERIFICATION.md
rm -f docs/PROJECT_COMPLETE.md
rm -f TEST_IMPLEMENTATION_COMPLETE.md WARM_START_COMPLETE.md
rm -f SYSTEM_VALIDATION_COMPLETE.md PRODUCTION_READINESS_REPORT.md
rm -f MVP_COMPLETE.md MVP_STATUS.md

# Historical plans
rm -f docs/30_DAY_EXECUTION_PLAN.md SEVEN_DAY_PRODUCTION_PUSH.md
rm -f PRODUCTION_START_CHECKLIST.md PROJECT_PROGRESS.md docs/PROJECT_PROGRESS.md

# Duplicate roadmaps
rm -f docs/04-roadmap.md docs/STRATEGIC_ROADMAP.md

# Old implementation plans
rm -f docs/memory-tracking-plan.md docs/nodejs-implementation-plan.md
rm -f docs/runtime-trait-design.md docs/storage-layer-design.md
rm -f docs/warm-start-implementation.md docs/dashboard-architecture.md

# Bug fixes & audits
rm -f docs/BUGFIX_STORAGE_RECREATION.md docs/DEPENDENCY_AUDIT.md

# Versioning docs (decision made)
rm -f docs/API_VERSIONING.md docs/FEATURE_VERSIONING.md docs/VERSIONING_DECISION.md

# Root level redundant files
rm -f DELIVERABLES_SUMMARY.md DOCUMENTATION_INDEX.md EXECUTOR_GUIDE.md
rm -f QUICK_REFERENCE.md COMPETITIVE_BENCHMARK_RESULTS.md LIVE_DEMO_RESULTS.md

echo "✅ Phase 1 complete"
echo ""

# Phase 2: Move Future Features to Archive
echo "📁 Phase 2: Archiving future feature documentation..."

mkdir -p docs/FUTURE_FEATURES

# Move future feature docs
mv -f docs/15-referral-program.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/16-annual-billing.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/17-usage-analytics.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/18-customer-lifetime-value.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/19-churn-analysis-prevention.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/20-payment-retry-logic.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/customer-portal.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/discount-codes-system.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/email-notifications.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/invoice-system.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/usage-alerts.md docs/FUTURE_FEATURES/ 2>/dev/null || true
mv -f docs/upgrade-prompts.md docs/FUTURE_FEATURES/ 2>/dev/null || true

echo "✅ Phase 2 complete"
echo ""

# Phase 3: Clean Dashboard (remove unused modular files)
echo "🧹 Phase 3: Cleaning dashboard folder..."

cd crates/api-server/dashboard

# Check if old files exist
if [ -d "css" ] || [ -d "js" ]; then
    echo "Found old modular dashboard files..."
    
    # Remove old CSS files (not used in embedded version)
    rm -rf css/ 2>/dev/null || true
    
    # Remove old JS files (not used in embedded version)
    rm -rf js/ 2>/dev/null || true
    
    # Remove assets if empty
    if [ -d "assets" ]; then
        if [ -z "$(ls -A assets)" ]; then
            rm -rf assets/
        fi
    fi
    
    echo "✅ Dashboard cleaned (old modular files removed)"
else
    echo "✅ Dashboard already clean"
fi

cd /workspaces/nanolambda
echo ""

# Phase 4: Clean test scripts (remove duplicates)
echo "🧪 Phase 4: Cleaning test scripts..."

rm -f test_dashboard_working.sh  # Duplicate of test_dashboard.sh

echo "✅ Phase 4 complete"
echo ""

# Summary
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║                    CLEANUP SUMMARY                              ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "Files deleted:"
echo "  • ~40 historical/outdated documentation files"
echo "  • Duplicate test scripts"
echo "  • Old dashboard modular files (css/, js/)"
echo ""
echo "Files archived:"
echo "  • 12 future feature docs → docs/FUTURE_FEATURES/"
echo ""
echo "Files kept:"
echo "  • ~20 essential production documentation files"
echo "  • README, QUICKSTART, CONTRIBUTING, CHANGELOG"
echo "  • index.html (embedded dashboard)"
echo ""
echo "✅ Cleanup complete!"
echo ""
echo "📊 Current documentation structure:"
find docs -type f -name "*.md" | wc -l | xargs echo "  • Documentation files:"
echo ""
echo "🔍 To review changes:"
echo "  git status"
echo ""
echo "💾 To commit cleanup:"
echo "  git add -A"
echo "  git commit -m 'Project cleanup: Remove outdated docs and files'"
echo ""
echo "⚠️  To rollback if needed:"
echo "  git checkout cleanup-backup"
echo ""
