# Mi7Soft HTTP 服务器集成测试脚本 (PowerShell)

Write-Host "=== Mi7Soft HTTP 服务器集成测试 ===" -ForegroundColor Green
Write-Host "测试时间: $(Get-Date)" -ForegroundColor Yellow
Write-Host ""

# 服务器地址
$ServerUrl = "http://localhost:8080"

# 测试函数
function Test-Endpoint {
    param(
        [string]$Method,
        [string]$Endpoint,
        [string]$Data = "",
        [string]$Description
    )
    
    Write-Host "测试: $Description" -ForegroundColor Cyan
    Write-Host "请求: $Method $Endpoint"
    
    try {
        if ($Method -eq "POST") {
            $headers = @{
                "Content-Type" = "application/json"
            }
            $response = Invoke-RestMethod -Uri "$ServerUrl$Endpoint" -Method $Method -Body $Data -Headers $headers -ErrorAction Stop
        } else {
            $response = Invoke-RestMethod -Uri "$ServerUrl$Endpoint" -Method $Method -ErrorAction Stop
        }
        
        Write-Host "响应码: 200" -ForegroundColor Green
        Write-Host "响应体: $($response | ConvertTo-Json -Compress)" -ForegroundColor White
    }
    catch {
        Write-Host "错误: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            Write-Host "响应码: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
        }
    }
    
    Write-Host "---"
    Write-Host ""
}

# 等待服务器启动
Write-Host "等待服务器启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

# 1. 测试 Hello 接口
Test-Endpoint -Method "GET" -Endpoint "/hello" -Description "Hello 接口测试"

# 2. 测试状态接口
Test-Endpoint -Method "GET" -Endpoint "/status" -Description "服务器状态查询"

# 3. 测试发送消息接口
Test-Endpoint -Method "POST" -Endpoint "/send" -Data '{"message": "测试消息1", "data": {"type": "test", "priority": 1}}' -Description "发送测试消息1"

Test-Endpoint -Method "POST" -Endpoint "/send" -Data '{"message": "测试消息2", "data": {"type": "urgent", "priority": 5}}' -Description "发送测试消息2"

Test-Endpoint -Method "POST" -Endpoint "/send" -Data '{"message": "Hello from HTTP API"}' -Description "发送简单消息"

# 4. 测试通用路径处理
Test-Endpoint -Method "GET" -Endpoint "/api/test" -Description "通用路径处理测试"

Test-Endpoint -Method "GET" -Endpoint "/custom/path/123" -Description "自定义路径测试"

# 5. 测试错误处理
Test-Endpoint -Method "POST" -Endpoint "/send" -Data '{"invalid": "json"}' -Description "错误请求测试"

Write-Host "=== 测试完成 ===" -ForegroundColor Green
Write-Host "请检查 worker 日志以确认消息是否被正确处理" -ForegroundColor Yellow
Write-Host "日志位置: logs\worker-*.log" -ForegroundColor Yellow