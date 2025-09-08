# CodeNothing 占位符清理测试脚本
# CodeNothing Placeholder Cleanup Test Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "CodeNothing 占位符清理测试脚本" -ForegroundColor Cyan
Write-Host "CodeNothing Placeholder Cleanup Test Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 检查编译器是否存在
$compilerPath = ".\target\release\CodeNothing.exe"
if (-not (Test-Path $compilerPath)) {
    Write-Host "❌ 错误: 找不到编译器 $compilerPath" -ForegroundColor Red
    Write-Host "❌ Error: Compiler not found at $compilerPath" -ForegroundColor Red
    Write-Host "请先运行: cargo build --release" -ForegroundColor Yellow
    Write-Host "Please run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ 编译器找到: $compilerPath" -ForegroundColor Green

# 测试文件列表
$testFiles = @(
    "placeholder_cleanup_test.cn",
    "comprehensive_placeholder_test.cn",
    "loop_test.cn"
)

$totalTests = 0
$passedTests = 0
$failedTests = 0

Write-Host "`n🧪 开始测试..." -ForegroundColor Yellow
Write-Host "🧪 Starting tests..." -ForegroundColor Yellow

foreach ($testFile in $testFiles) {
    Write-Host "`n----------------------------------------" -ForegroundColor Cyan
    Write-Host "测试文件: $testFile" -ForegroundColor Cyan
    Write-Host "Testing file: $testFile" -ForegroundColor Cyan
    Write-Host "----------------------------------------" -ForegroundColor Cyan
    
    $totalTests++
    
    # 检查测试文件是否存在
    if (-not (Test-Path $testFile)) {
        Write-Host "⚠️  警告: 测试文件 $testFile 不存在，跳过" -ForegroundColor Yellow
        Write-Host "⚠️  Warning: Test file $testFile not found, skipping" -ForegroundColor Yellow
        continue
    }
    
    # 编译测试
    Write-Host "🔨 编译测试..." -ForegroundColor Blue
    Write-Host "🔨 Compiling..." -ForegroundColor Blue
    
    $compileResult = & $compilerPath --build $testFile 2>&1
    $compileExitCode = $LASTEXITCODE
    
    if ($compileExitCode -eq 0) {
        Write-Host "✅ 编译成功" -ForegroundColor Green
        Write-Host "✅ Compilation successful" -ForegroundColor Green
        
        # 运行测试
        Write-Host "🚀 运行测试..." -ForegroundColor Blue
        Write-Host "🚀 Running test..." -ForegroundColor Blue
        
        $bytecodeFile = $testFile -replace "\.cn$", ".comcn"
        $runResult = & $compilerPath --run $bytecodeFile 2>&1
        $runExitCode = $LASTEXITCODE
        
        if ($runExitCode -eq 0) {
            Write-Host "✅ 运行成功" -ForegroundColor Green
            Write-Host "✅ Execution successful" -ForegroundColor Green
            $passedTests++
            
            # 显示部分输出
            if ($runResult) {
                Write-Host "📄 输出摘要:" -ForegroundColor Magenta
                Write-Host "📄 Output summary:" -ForegroundColor Magenta
                $runResult | Select-Object -First 5 | ForEach-Object {
                    Write-Host "   $_" -ForegroundColor Gray
                }
                if ($runResult.Count -gt 5) {
                    Write-Host "   ... (更多输出)" -ForegroundColor Gray
                    Write-Host "   ... (more output)" -ForegroundColor Gray
                }
            }
        } else {
            Write-Host "❌ 运行失败 (退出码: $runExitCode)" -ForegroundColor Red
            Write-Host "❌ Execution failed (exit code: $runExitCode)" -ForegroundColor Red
            Write-Host "错误输出:" -ForegroundColor Red
            Write-Host "Error output:" -ForegroundColor Red
            $runResult | ForEach-Object { Write-Host "   $_" -ForegroundColor Red }
            $failedTests++
        }
    } else {
        Write-Host "❌ 编译失败 (退出码: $compileExitCode)" -ForegroundColor Red
        Write-Host "❌ Compilation failed (exit code: $compileExitCode)" -ForegroundColor Red
        Write-Host "错误输出:" -ForegroundColor Red
        Write-Host "Error output:" -ForegroundColor Red
        $compileResult | ForEach-Object { Write-Host "   $_" -ForegroundColor Red }
        $failedTests++
    }
}

# 性能测试
Write-Host "`n🏃‍♂️ 性能测试..." -ForegroundColor Yellow
Write-Host "🏃‍♂️ Performance test..." -ForegroundColor Yellow

if (Test-Path "comprehensive_placeholder_test.cn") {
    Write-Host "运行性能分析..." -ForegroundColor Blue
    Write-Host "Running performance analysis..." -ForegroundColor Blue
    
    $perfResult = & $compilerPath --stats comprehensive_placeholder_test.cn 2>&1
    $perfExitCode = $LASTEXITCODE
    
    if ($perfExitCode -eq 0) {
        Write-Host "✅ 性能测试完成" -ForegroundColor Green
        Write-Host "✅ Performance test completed" -ForegroundColor Green
        
        # 提取关键性能指标
        $perfResult | Where-Object { $_ -match "Instructions executed|Memory efficiency|Hit rate" } | ForEach-Object {
            Write-Host "📊 $_" -ForegroundColor Cyan
        }
    } else {
        Write-Host "❌ 性能测试失败" -ForegroundColor Red
        Write-Host "❌ Performance test failed" -ForegroundColor Red
    }
}

# 测试总结
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "测试总结 / Test Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

Write-Host "总测试数 / Total tests: $totalTests" -ForegroundColor White
Write-Host "通过测试 / Passed tests: $passedTests" -ForegroundColor Green
Write-Host "失败测试 / Failed tests: $failedTests" -ForegroundColor Red

if ($failedTests -eq 0) {
    Write-Host "`n🎉 所有测试通过！" -ForegroundColor Green
    Write-Host "🎉 All tests passed!" -ForegroundColor Green
    Write-Host "✅ 占位符清理验证成功" -ForegroundColor Green
    Write-Host "✅ Placeholder cleanup verification successful" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n⚠️  有测试失败，请检查上述错误" -ForegroundColor Yellow
    Write-Host "⚠️  Some tests failed, please check the errors above" -ForegroundColor Yellow
    exit 1
}
