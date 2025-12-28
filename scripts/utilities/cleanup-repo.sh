#!/bin/bash
# Automated Project Cleanup Script
# Cleans up nanolambda repository structure

set -e

echo "🧹 NanoLambda Project Cleanup"
echo "=============================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Must be run from project root!"
    exit 1
fi

echo "This script will:"
echo "  1. Remove database files from git"
echo "  2. Remove log files from git"
echo "  3. Remove test results from git"
echo "  4. Organize documentation"
echo "  5. Organize scripts"
echo "  6. Update .gitignore"
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
# Priority 1: Remove Sensitive Files
# =====================================

echo "Priority 1: Removing sensitive files from git..."

# Remove database files
if git ls-files | grep -q "\.db$"; then
    print_warning "Removing database files from git..."
    git rm --cached nanolambda.db 2>/dev/null || true
    git rm --cached nanolambda.db.usage.db 2>/dev/null || true
    git rm --cached crates/api-server/dashboard/nanolambda.db 2>/dev/null || true
    git rm --cached crates/api-server/dashboard/nanolambda.db.usage.db 2>/dev/null || true
    print_status "Database files removed from git tracking"
else
    print_status "No database files tracked in git"
fi

# Remove log files
if git ls-files | grep -q "\.log$"; then
    print_warning "Removing log files from git..."
    git rm --cached *.log 2>/dev/null || true
    git rm --cached -r test-suite/results/ 2>/dev/null || true
    print_status "Log files removed from git tracking"
else
    print_status "No log files tracked in git"
fi

# =====================================
# Priority 2: Update .gitignore
# =====================================

echo ""
echo "Priority 2: Updating .gitignore..."

# Backup original .gitignore
cp .gitignore .gitignore.backup

# Add additional ignores if not present
cat >> .gitignore << 'EOF'

# Database files
*.db
*.db-*
nanolambda.db*
*.db.usage.db

# Test results
test-suite/results/
test-results.txt

# Additional logs
tier_test.log
test_server.log
test_output.log
server.log
server_debug.log
final_test.log

EOF

print_status ".gitignore updated"

# =====================================
# Priority 3: Organize Documentation
# =====================================

echo ""
echo "Priority 3: Organizing documentation..."

# Create archive directories
mkdir -p docs/archive
mkdir -p docs/milestones
mkdir -p docs/testing

print_status "Created docs/archive, docs/milestones, docs/testing"

# Move historical milestone docs
print_warning "Moving historical milestone documents..."
[ -f "MVP_COMPLETE.md" ] && mv MVP_COMPLETE.md docs/milestones/
[ -f "MVP_STATUS.md" ] && mv MVP_STATUS.md docs/milestones/
[ -f "WARM_START_COMPLETE.md" ] && mv WARM_START_COMPLETE.md docs/milestones/
print_status "Milestone docs moved"

# Move outdated/redundant docs to archive
print_warning "Archiving redundant documentation..."
[ -f "WORK_COMPLETION_SUMMARY.md" ] && mv WORK_COMPLETION_SUMMARY.md docs/archive/
[ -f "TODO_IMPLEMENTATION_REPORT.md" ] && mv TODO_IMPLEMENTATION_REPORT.md docs/archive/
[ -f "PROJECT_PROGRESS.md" ] && mv PROJECT_PROGRESS.md docs/archive/
[ -f "PROJECT_SUMMARY.md" ] && mv PROJECT_SUMMARY.md docs/archive/
[ -f "PRODUCTION_READINESS_REPORT.md" ] && mv PRODUCTION_READINESS_REPORT.md docs/archive/
[ -f "PROD_READY_REPORT.md" ] && mv PROD_READY_REPORT.md docs/archive/
[ -f "PRE_PRODUCTION_COMPLETE.md" ] && mv PRE_PRODUCTION_COMPLETE.md docs/archive/
[ -f "PRODUCTION_IMPROVEMENTS.md" ] && mv PRODUCTION_IMPROVEMENTS.md docs/archive/
print_status "Redundant docs archived"

# Move test documentation
print_warning "Organizing test documentation..."
[ -f "TEST_IMPLEMENTATION_COMPLETE.md" ] && mv TEST_IMPLEMENTATION_COMPLETE.md docs/testing/
[ -f "TESTING_GUIDE.md" ] && mv TESTING_GUIDE.md docs/testing/
[ -f "TEST_COVERAGE_GUIDE.md" ] && mv TEST_COVERAGE_GUIDE.md docs/testing/
[ -f "TEST_SUITE.md" ] && mv TEST_SUITE.md docs/testing/
print_status "Test docs organized"

# =====================================
# Priority 4: Organize Scripts
# =====================================

echo ""
echo "Priority 4: Organizing scripts..."

# Create script directories
mkdir -p scripts/testing
mkdir -p scripts/analysis
mkdir -p scripts/utilities

print_status "Created scripts directories"

# Move Python test scripts
print_warning "Moving test scripts..."
[ -f "test-webhooks.py" ] && mv test-webhooks.py scripts/testing/
[ -f "test_executor.py" ] && mv test_executor.py scripts/testing/
[ -f "test-email-notifications.py" ] && mv test-email-notifications.py scripts/testing/
[ -f "test-metered-billing.py" ] && mv test-metered-billing.py scripts/testing/
print_status "Python test scripts moved"

# Move analysis scripts
print_warning "Moving analysis scripts..."
[ -f "coverage-analysis.sh" ] && mv coverage-analysis.sh scripts/analysis/
[ -f "find-untested-code.sh" ] && mv find-untested-code.sh scripts/analysis/
print_status "Analysis scripts moved"

# Move utility scripts
print_warning "Moving utility scripts..."
[ -f "cleanup-project.sh" ] && mv cleanup-project.sh scripts/utilities/
print_status "Utility scripts moved"

# =====================================
# Priority 5: Remove Unused Code
# =====================================

echo ""
echo "Priority 5: Checking for unused code..."

# Check if demo code is used
if [ -d "crates/api-server/src/demo" ]; then
    if ! grep -r "mod demo" crates/api-server/src/*.rs > /dev/null 2>&1; then
        print_warning "Demo code appears unused. Moving to examples..."
        mkdir -p examples/archived
        mv crates/api-server/src/demo examples/archived/demo-dashboard
        print_status "Demo code moved to examples/archived/"
    else
        print_status "Demo code is in use, keeping it"
    fi
else
    print_status "No demo code found"
fi

# =====================================
# Priority 6: Clean up test results
# =====================================

echo ""
echo "Priority 6: Cleaning test results..."

if [ -d "test-suite/results" ]; then
    print_warning "Removing old test results..."
    rm -rf test-suite/results/
    print_status "Test results removed"
else
    print_status "No test results to clean"
fi

# =====================================
# Summary
# =====================================

echo ""
echo "=============================="
echo "✅ Cleanup Complete!"
echo "=============================="
echo ""
echo "Summary of changes:"
echo "  - Database files removed from git"
echo "  - Log files removed from git"
echo "  - Documentation organized into docs/archive, docs/milestones, docs/testing"
echo "  - Scripts organized into scripts/testing, scripts/analysis, scripts/utilities"
echo "  - Unused code moved to examples/archived"
echo "  - .gitignore updated"
echo ""
echo "Files modified:"
git status --short
echo ""
echo "Next steps:"
echo "  1. Review changes with: git status"
echo "  2. Review documentation structure in docs/"
echo "  3. Commit changes: git add -A && git commit -m 'chore: Clean up project structure'"
echo "  4. Push to remote: git push origin main"
echo ""
echo "Backup created: .gitignore.backup"
echo ""
print_status "Done! Review and commit when ready."
