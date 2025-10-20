# 多进程消息队列演示脚本
Write-Host "🚀 启动多进程消息队列演示" -ForegroundColor Green

# 编译项目
Write-Host "🔨 编译项目..." -ForegroundColor Yellow
wsl bash -c '. ~/.cargo/env && cargo build --examples'

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 编译成功" -ForegroundColor Green

# 启动多个worker进程
Write-Host "🔧 启动 3 个 Worker 进程..." -ForegroundColor Yellow

$worker1 = Start-Process -FilePath "wsl" -ArgumentList "bash", "-c", "'. ~/.cargo/env && cd /mnt/d/work/mi7/mi7soft-rust && ./target/debug/examples/worker worker-1'" -PassThru -WindowStyle Minimized
$worker2 = Start-Process -FilePath "wsl" -ArgumentList "bash", "-c", "'. ~/.cargo/env && cd /mnt/d/work/mi7/mi7soft-rust && ./target/debug/examples/worker worker-2'" -PassThru -WindowStyle Minimized  
$worker3 = Start-Process -FilePath "wsl" -ArgumentList "bash", "-c", "'. ~/.cargo/env && cd /mnt/d/work/mi7/mi7soft-rust && ./target/debug/examples/worker worker-3'" -PassThru -WindowStyle Minimized

Write-Host "✅ Worker 进程已启动" -ForegroundColor Green
Write-Host "   Worker 1 PID: $($worker1.Id)" -ForegroundColor Cyan
Write-Host "   Worker 2 PID: $($worker2.Id)" -ForegroundColor Cyan  
Write-Host "   Worker 3 PID: $($worker3.Id)" -ForegroundColor Cyan

# 等待一下让worker启动
Start-Sleep -Seconds 2

# 启动生产者
Write-Host "📝 启动生产者..." -ForegroundColor Yellow
wsl bash -c '. ~/.cargo/env && cd /mnt/d/work/mi7/mi7soft-rust && ./target/debug/examples/producer'

Write-Host "🏁 生产者完成" -ForegroundColor Green

# 等待worker处理完成
Write-Host "⏳ 等待 Worker 处理完成..." -ForegroundColor Yellow
Start-Sleep -Seconds 35

# 检查进程状态
Write-Host "📊 检查进程状态..." -ForegroundColor Yellow

$processes = @($worker1, $worker2, $worker3)
foreach ($proc in $processes) {
    if (-not $proc.HasExited) {
        Write-Host "⚠️  Worker PID $($proc.Id) 仍在运行，正在终止..." -ForegroundColor Yellow
        $proc.Kill()
        $proc.WaitForExit()
    }
    Write-Host "✅ Worker PID $($proc.Id) 已退出" -ForegroundColor Green
}

Write-Host "🎉 演示完成！" -ForegroundColor Green
Write-Host "💡 提示: 你可以手动运行以下命令来测试:" -ForegroundColor Cyan
Write-Host "   生产者: wsl bash -c '. ~/.cargo/env && cargo run --example producer'" -ForegroundColor White
Write-Host "   消费者: wsl bash -c '. ~/.cargo/env && cargo run --example worker worker-1'" -ForegroundColor White