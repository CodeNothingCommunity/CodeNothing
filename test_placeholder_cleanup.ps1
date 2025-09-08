# 简化的占位符清理测试脚本
Write-Host "CodeNothing 占位符清理测试" -ForegroundColor Cyan

# 检查编译器
$compiler = ".\target\release\CodeNothing.exe"
if (-not (Test-Path $compiler)) {
    Write-Host "编译器未找到: $compiler" -ForegroundColor Red
    exit 1
}

Write-Host "编译器找到: $compiler" -ForegroundColor Green

# 测试文件
$tests = @(
    "placeholder_cleanup_test.cn",
    "comprehensive_placeholder_test.cn"
)

$passed = 0
$total = 0

foreach ($test in $tests) {
    $total++
    Write-Host "`n测试: $test" -ForegroundColor Yellow
    
    if (-not (Test-Path $test)) {
        Write-Host "文件不存在: $test" -ForegroundColor Red
        continue
    }
    
    # 编译
    Write-Host "编译中..." -ForegroundColor Blue
    $result = & $compiler --build $test
    if ($LASTEXITCODE -eq 0) {
        Write-Host "编译成功" -ForegroundColor Green
        
        # 运行
        $bytecode = $test -replace "\.cn$", ".comcn"
        Write-Host "运行中..." -ForegroundColor Blue
        $result = & $compiler --run $bytecode
        if ($LASTEXITCODE -eq 0) {
            Write-Host "运行成功" -ForegroundColor Green
            $passed++
        } else {
            Write-Host "运行失败" -ForegroundColor Red
        }
    } else {
        Write-Host "编译失败" -ForegroundColor Red
    }
}

Write-Host "`n测试总结:" -ForegroundColor Cyan
Write-Host "总数: $total" -ForegroundColor White
Write-Host "通过: $passed" -ForegroundColor Green
Write-Host "失败: $($total - $passed)" -ForegroundColor Red

if ($passed -eq $total) {
    Write-Host "`n所有测试通过!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n有测试失败" -ForegroundColor Yellow
    exit 1
}
