# CodeNothing Include System Comprehensive Test Script
# 测试当前include系统的所有功能

Write-Host "🚀 CodeNothing Include System Comprehensive Test" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Green

# 测试计数器
$totalTests = 0
$passedTests = 0
$failedTests = 0

function Test-CompileAndRun {
    param(
        [string]$TestName,
        [string]$SourceFile,
        [string]$ExpectedOutput = ""
    )
    
    $global:totalTests++
    Write-Host "`n📝 Testing: $TestName" -ForegroundColor Yellow
    Write-Host "   Source: $SourceFile" -ForegroundColor Gray
    
    # 编译
    Write-Host "   🔨 Compiling..." -ForegroundColor Cyan
    $compileResult = & cargo run -- --build $SourceFile 2>&1

    if ($LASTEXITCODE -ne 0) {
        Write-Host "   ❌ Compilation failed!" -ForegroundColor Red
        Write-Host "   Error: $compileResult" -ForegroundColor Red
        $global:failedTests++
        return $false
    }

    # 运行
    $comcnFile = $SourceFile -replace '\.cn$', '.comcn'
    if (Test-Path $comcnFile) {
        Write-Host "   ▶️  Running..." -ForegroundColor Cyan
        $runResult = & cargo run -- --run $comcnFile 2>&1
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "   ✅ Test passed!" -ForegroundColor Green
            Write-Host "   Output: $runResult" -ForegroundColor Gray
            $global:passedTests++
            return $true
        } else {
            Write-Host "   ❌ Runtime error!" -ForegroundColor Red
            Write-Host "   Error: $runResult" -ForegroundColor Red
            $global:failedTests++
            return $false
        }
    } else {
        Write-Host "   ❌ Bytecode file not found!" -ForegroundColor Red
        $global:failedTests++
        return $false
    }
}

# 测试1: 基本Include功能
Write-Host "`n🔍 Test Group 1: Basic Include Functionality" -ForegroundColor Magenta

Test-CompileAndRun "Basic Include" "simple_include_test.cn"
Test-CompileAndRun "Include with Utils" "include_test_main.cn"

# 测试2: 通配符匹配
Write-Host "`n🔍 Test Group 2: Wildcard Matching" -ForegroundColor Magenta

Test-CompileAndRun "Simple Wildcard" "test_simple_wildcard.cn"
Test-CompileAndRun "Wildcard Utils Clean" "test_wildcard_clean.cn"
Test-CompileAndRun "Wildcard Include" "test_wildcard_include.cn"

# 测试3: 选择性导入
Write-Host "`n🔍 Test Group 3: Selective Import" -ForegroundColor Magenta

Test-CompileAndRun "Selective Include" "selective_include_test.cn"
Test-CompileAndRun "Selective Include Only" "test_selective_include_only.cn"

# 测试4: 高级功能
Write-Host "`n🔍 Test Group 4: Advanced Features" -ForegroundColor Magenta

Test-CompileAndRun "Enhanced Include" "test_enhanced_include.cn"
Test-CompileAndRun "Include Comprehensive" "test_include_comprehensive.cn"

# 测试5: 递归和复杂模式
Write-Host "`n🔍 Test Group 5: Complex Patterns" -ForegroundColor Magenta

if (Test-Path "test_recursive_wildcard.cn") {
    Test-CompileAndRun "Recursive Wildcard" "test_recursive_wildcard.cn"
}

if (Test-Path "test_include_edge_cases.cn") {
    Test-CompileAndRun "Edge Cases" "test_include_edge_cases.cn"
}

# 测试6: 别名功能
Write-Host "`n🔍 Test Group 6: Alias Features" -ForegroundColor Magenta

if (Test-Path "simple_alias_test.cn") {
    Test-CompileAndRun "Simple Alias" "simple_alias_test.cn"
}

# 性能测试
Write-Host "`n🔍 Test Group 7: Performance Test" -ForegroundColor Magenta

Write-Host "   📊 Testing compilation performance..." -ForegroundColor Cyan
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

# 编译一个复杂的include文件
if (Test-Path "test_wildcard_clean.cn") {
    $compileResult = & cargo run -- --build test_wildcard_clean.cn 2>&1
    $stopwatch.Stop()

    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ✅ Performance test passed!" -ForegroundColor Green
        Write-Host "   ⏱️  Compilation time: $($stopwatch.ElapsedMilliseconds)ms" -ForegroundColor Gray
    } else {
        Write-Host "   ❌ Performance test failed!" -ForegroundColor Red
    }
}

# 清理测试文件
Write-Host "`n🧹 Cleaning up test artifacts..." -ForegroundColor Cyan
Get-ChildItem -Filter "*.comcn" | Remove-Item -Force -ErrorAction SilentlyContinue

# 测试结果汇总
Write-Host "`n📊 Test Results Summary" -ForegroundColor Green
Write-Host "======================" -ForegroundColor Green
Write-Host "Total Tests: $totalTests" -ForegroundColor White
Write-Host "Passed: $passedTests" -ForegroundColor Green
Write-Host "Failed: $failedTests" -ForegroundColor Red

if ($failedTests -eq 0) {
    Write-Host "`n🎉 All tests passed! Include system is working correctly." -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n⚠️  Some tests failed. Please check the output above." -ForegroundColor Yellow
    exit 1
}
