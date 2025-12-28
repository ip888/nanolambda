#!/bin/bash

# Individual Language/Scenario Test Runner

set -e

LANGUAGE=${1:-python}
SCENARIO=${2:-sanity}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get or create API key
if [ -z "$API_KEY" ]; then
    if [ -f "$SCRIPT_DIR/results/latest/api_key.txt" ]; then
        API_KEY=$(cat "$SCRIPT_DIR/results/latest/api_key.txt")
    else
        API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
            -H "Content-Type: application/json" \
            -d '{"name": "test-'$(date +%s)'", "expires_at": null}' | \
            grep -o '"key":"[^"]*' | cut -d'"' -f4)
    fi
    export API_KEY
fi

deploy_function() {
    local name=$1
    local runtime=$2
    local handler=$3
    local code_dir=$4
    
    echo "Deploying function: $name ($runtime)"
    
    # Create function
    curl -s -X POST http://localhost:8080/functions \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\": \"$name\",
            \"runtime\": \"$runtime\",
            \"handler\": \"$handler\",
            \"memory_mb\": 256,
            \"timeout_seconds\": 30
        }" > /dev/null
    
    # Upload code
    cd "$code_dir"
    zip -q -r /tmp/function_${name}.zip .
    curl -s -X POST "http://localhost:8080/functions/$name/code" \
        -H "Authorization: Bearer $API_KEY" \
        -F "file=@/tmp/function_${name}.zip" > /dev/null
    
    echo "✓ Deployed: $name"
}

invoke_function() {
    local name=$1
    local payload=$2
    local count=${3:-1}
    
    for i in $(seq 1 $count); do
        curl -s -X POST "http://localhost:8080/functions/$name/invoke" \
            -H "Authorization: Bearer $API_KEY" \
            -H "Content-Type: application/json" \
            -d "$payload" > /dev/null
    done
}

# Python tests
run_python_tests() {
    echo "Running Python tests..."
    
    case $SCENARIO in
        sanity)
            deploy_function "python-hello" "python3.11" "handler.handler" \
                "$SCRIPT_DIR/functions/python/rest-api"
            invoke_function "python-hello" '{"test": "data"}' 10
            ;;
        load)
            deploy_function "python-api" "python3.11" "handler.handler" \
                "$SCRIPT_DIR/functions/python/rest-api"
            deploy_function "python-processor" "python3.11" "handler.handler" \
                "$SCRIPT_DIR/functions/python/data-processing"
            
            # High load
            for i in {1..100}; do
                invoke_function "python-api" '{"request": '$i'}' 1 &
            done
            wait
            ;;
    esac
}

# Node.js tests
run_nodejs_tests() {
    echo "Running Node.js tests..."
    
    case $SCENARIO in
        sanity)
            deploy_function "nodejs-hello" "nodejs20" "index.handler" \
                "$SCRIPT_DIR/functions/nodejs/express-api"
            invoke_function "nodejs-hello" '{"test": "data"}' 10
            ;;
        load)
            deploy_function "nodejs-api" "nodejs20" "index.handler" \
                "$SCRIPT_DIR/functions/nodejs/express-api"
            deploy_function "nodejs-processor" "nodejs20" "index.handler" \
                "$SCRIPT_DIR/functions/nodejs/image-processor"
            
            # High load
            for i in {1..100}; do
                invoke_function "nodejs-api" '{"request": '$i'}' 1 &
            done
            wait
            ;;
    esac
}

# Java tests
run_java_tests() {
    echo "Running Java tests..."
    
    case $SCENARIO in
        sanity)
            deploy_function "java-hello" "java17" "Handler::handleRequest" \
                "$SCRIPT_DIR/functions/java/spring-boot-api"
            invoke_function "java-hello" '{"test": "data"}' 10
            ;;
        load)
            deploy_function "java-api" "java17" "Handler::handleRequest" \
                "$SCRIPT_DIR/functions/java/spring-boot-api"
            
            # High load
            for i in {1..100}; do
                invoke_function "java-api" '{"request": '$i'}' 1 &
            done
            wait
            ;;
    esac
}

# Main execution
case $LANGUAGE in
    python)
        run_python_tests
        ;;
    nodejs)
        run_nodejs_tests
        ;;
    java)
        run_java_tests
        ;;
    mixed)
        run_python_tests
        run_nodejs_tests
        run_java_tests
        ;;
    *)
        echo "Unknown language: $LANGUAGE"
        exit 1
        ;;
esac

echo "✓ Tests completed successfully"
