# CodeNothing增强Include系统测试脚本
# Enhanced Include System Test Script

Write-Host "=== CodeNothing增强Include系统测试 ===" -ForegroundColor Cyan
Write-Host ""

# 检查编译器是否存在
if (-not (Test-Path ".\target\debug\CodeNothing.exe")) {
    Write-Host "❌ 编译器不存在，请先运行 cargo build" -ForegroundColor Red
    exit 1
}

$testResults = @()
$totalTests = 0
$passedTests = 0

function Test-Compilation {
    param(
        [string]$TestName,
        [string]$SourceFile,
        [bool]$ShouldSucceed = $true
    )
    
    $global:totalTests++
    Write-Host "🧪 测试: $TestName" -ForegroundColor Yellow
    Write-Host "   文件: $SourceFile"
    
    $result = & ".\target\debug\CodeNothing.exe" --build $SourceFile 2>&1
    $success = $LASTEXITCODE -eq 0
    
    if ($ShouldSucceed -eq $success) {
        Write-Host "   [PASS] 通过" -ForegroundColor Green
        $global:passedTests++
        $global:testResults += @{Name=$TestName; Status="PASS"; File=$SourceFile}
    } else {
        $expectedResult = if ($ShouldSucceed) { "成功" } else { "失败" }
        $actualResult = if ($success) { "成功" } else { "失败" }
        Write-Host "   [FAIL] 失败 (期望: $expectedResult, 实际: $actualResult)" -ForegroundColor Red
        $global:testResults += @{Name=$TestName; Status="FAIL"; File=$SourceFile}
        Write-Host "   输出: $result" -ForegroundColor Gray
    }
    Write-Host ""
}

function Test-Execution {
    param(
        [string]$TestName,
        [string]$BytecodeFile,
        [string]$ExpectedOutput
    )
    
    $global:totalTests++
    Write-Host "🚀 运行测试: $TestName" -ForegroundColor Yellow
    Write-Host "   字节码: $BytecodeFile"
    
    if (-not (Test-Path $BytecodeFile)) {
        Write-Host "   [FAIL] 字节码文件不存在" -ForegroundColor Red
        $global:testResults += @{Name=$TestName; Status="FAIL"; File=$BytecodeFile}
        return
    }

    $output = & ".\target\debug\CodeNothing.exe" --run $BytecodeFile 2>&1
    $success = $LASTEXITCODE -eq 0

    if ($success -and $output -like "*$ExpectedOutput*") {
        Write-Host "   [PASS] 通过" -ForegroundColor Green
        $global:passedTests++
        $global:testResults += @{Name=$TestName; Status="PASS"; File=$BytecodeFile}
    } else {
        Write-Host "   [FAIL] 失败" -ForegroundColor Red
        $global:testResults += @{Name=$TestName; Status="FAIL"; File=$BytecodeFile}
        Write-Host "   期望输出包含: $ExpectedOutput" -ForegroundColor Gray
        Write-Host "   实际输出: $output" -ForegroundColor Gray
    }
    Write-Host ""
}

# 1. 基本Include功能测试
Write-Host "📁 1. 基本Include功能测试" -ForegroundColor Magenta
Test-Compilation "基本Include" "debug_wildcard.cn"
Test-Execution "基本Include运行" "debug_wildcard.comcn" "Debug Wildcard Test"

# 2. 别名功能测试
Write-Host "🏷️ 2. 别名功能测试" -ForegroundColor Magenta
Test-Compilation "别名导入" "simple_alias_test.cn"
Test-Execution "别名导入运行" "simple_alias_test.comcn" "Testing alias"

# 3. 通配符功能测试
Write-Host "🔍 3. 通配符功能测试" -ForegroundColor Magenta
Test-Compilation "通配符匹配" "test_wildcard_clean.cn"
Test-Execution "通配符匹配运行" "test_wildcard_clean.comcn" "Clean Wildcard Test"

# 4. 选择性导入测试
Write-Host "📋 4. 选择性导入测试" -ForegroundColor Magenta
Test-Compilation "选择性导入" "test_edge_cases.cn"
Test-Execution "选择性导入运行" "test_edge_cases.comcn" "Edge Cases Test"

# 5. 递归通配符测试
Write-Host "🔄 5. 递归通配符测试" -ForegroundColor Magenta
Test-Compilation "递归通配符" "test_recursive_wildcard.cn"
Test-Execution "递归通配符运行" "test_recursive_wildcard.comcn" "Recursive Wildcard Test"

# 6. 错误处理测试
Write-Host "⚠️ 6. 错误处理测试" -ForegroundColor Magenta
Test-Compilation "不存在文件错误" "test_error_cases.cn" $false

# 7. 静态链接验证测试
Write-Host "🔗 7. 静态链接验证测试" -ForegroundColor Magenta
Write-Host "🧪 测试: 静态链接验证" -ForegroundColor Yellow

# 备份文件
if (Test-Path "wildcard_utils_a.cn") {
    Copy-Item "wildcard_utils_a.cn" "wildcard_utils_a.cn.test_backup"
}
if (Test-Path "wildcard_utils_b.cn") {
    Copy-Item "wildcard_utils_b.cn" "wildcard_utils_b.cn.test_backup"
}

# 删除源文件
Remove-Item "wildcard_utils_a.cn" -ErrorAction SilentlyContinue
Remove-Item "wildcard_utils_b.cn" -ErrorAction SilentlyContinue

# 测试字节码是否仍能运行
$output = & ".\target\debug\CodeNothing.exe" --run "test_compilation_logic.comcn" 2>&1
$success = $LASTEXITCODE -eq 0

if ($success -and $output -like "*Compilation Logic Test*") {
    Write-Host "   [PASS] 静态链接验证通过" -ForegroundColor Green
    $global:passedTests++
} else {
    Write-Host "   [FAIL] 静态链接验证失败" -ForegroundColor Red
}
$global:totalTests++

# 恢复文件
if (Test-Path "wildcard_utils_a.cn.test_backup") {
    Move-Item "wildcard_utils_a.cn.test_backup" "wildcard_utils_a.cn"
}
if (Test-Path "wildcard_utils_b.cn.test_backup") {
    Move-Item "wildcard_utils_b.cn.test_backup" "wildcard_utils_b.cn"
}

# 测试结果总结
Write-Host ""
Write-Host "📊 测试结果总结" -ForegroundColor Cyan
Write-Host "=================" -ForegroundColor Cyan
Write-Host "总测试数: $totalTests"
Write-Host "通过测试: $passedTests" -ForegroundColor Green
Write-Host "失败测试: $($totalTests - $passedTests)" -ForegroundColor Red
Write-Host "成功率: $([math]::Round(($passedTests / $totalTests) * 100, 2))%"

Write-Host ""
Write-Host "详细结果:" -ForegroundColor Yellow
foreach ($result in $testResults) {
    $status = if ($result.Status -eq "PASS") { "[PASS]" } else { "[FAIL]" }
    Write-Host "  $status $($result.Name) ($($result.File))"
}

Write-Host ""
if ($passedTests -eq $totalTests) {
    Write-Host "🎉 所有测试通过！增强Include系统工作正常！" -ForegroundColor Green
} else {
    Write-Host "⚠️ 部分测试失败，请检查实现。" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== 测试完成 ===" -ForegroundColor Cyan
