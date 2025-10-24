#!/bin/bash

# Mi7Soft HTTP 服务器集成测试脚本

echo "=== Mi7Soft HTTP 服务器集成测试 ==="
echo "测试时间: $(date)"
echo

# 服务器地址
SERVER_URL="http://localhost:8080"

# 测试函数
test_endpoint() {
    local method=$1
    local endpoint=$2
    local data=$3
    local description=$4
    
    echo "测试: $description"
    echo "请求: $method $endpoint"
    
    if [ "$method" = "POST" ]; then
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$SERVER_URL$endpoint")
    else
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
            "$SERVER_URL$endpoint")
    fi
    
    http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
    body=$(echo "$response" | sed '/HTTP_CODE:/d')
    
    echo "响应码: $http_code"
    echo "响应体: $body"
    echo "---"
    echo
}

# 等待服务器启动
echo "等待服务器启动..."
sleep 2

# 1. 测试 Hello 接口
test_endpoint "GET" "/hello" "" "Hello 接口测试"

# 2. 测试状态接口
test_endpoint "GET" "/status" "" "服务器状态查询"

# 3. 测试发送消息接口
test_endpoint "POST" "/send" '{"message": "测试消息1", "data": {"type": "test", "priority": 1}}' "发送测试消息1"

test_endpoint "POST" "/send" '{"message": "测试消息2", "data": {"type": "urgent", "priority": 5}}' "发送测试消息2"

test_endpoint "POST" "/send" '{"message": "Hello from HTTP API"}' "发送简单消息"

# 4. 测试通用路径处理
test_endpoint "GET" "/api/test" "" "通用路径处理测试"

test_endpoint "GET" "/custom/path/123" "" "自定义路径测试"

# 5. 测试错误处理
test_endpoint "POST" "/send" '{"invalid": "json"}' "错误请求测试"

echo "=== 测试完成 ==="
echo "请检查 worker 日志以确认消息是否被正确处理"
echo "日志位置: logs/worker-*.log"