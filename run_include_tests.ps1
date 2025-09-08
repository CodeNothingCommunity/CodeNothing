# PowerShell script to run comprehensive include tests
Write-Host "=== CodeNothing Include System Test Suite ===" -ForegroundColor Green

# Build the project first
Write-Host "`nBuilding CodeNothing..." -ForegroundColor Yellow
cargo build --release --quiet
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "Build successful!" -ForegroundColor Green

# Test files to run
$testFiles = @(
    "simple_main.cn",
    "include_test_main.cn", 
    "selective_include_test.cn",
    "test_include_comprehensive.cn",
    "test_include_edge_cases.cn"
)

$passedTests = 0
$totalTests = $testFiles.Count

Write-Host "`nRunning $totalTests include tests..." -ForegroundColor Yellow

foreach ($testFile in $testFiles) {
    Write-Host "`n--- Testing: $testFile ---" -ForegroundColor Cyan
    
    if (Test-Path $testFile) {
        $result = & ".\target\release\CodeNothing.exe" $testFile --verbose
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ $testFile PASSED" -ForegroundColor Green
            $passedTests++
        } else {
            Write-Host "❌ $testFile FAILED" -ForegroundColor Red
            Write-Host "Error output:" -ForegroundColor Red
            Write-Host $result -ForegroundColor Red
        }
    } else {
        Write-Host "⚠️  $testFile not found, skipping..." -ForegroundColor Yellow
    }
}

Write-Host "`n=== Test Results ===" -ForegroundColor Green
Write-Host "Passed: $passedTests / $totalTests" -ForegroundColor Green

if ($passedTests -eq $totalTests) {
    Write-Host "🎉 All include tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ Some tests failed!" -ForegroundColor Red
    exit 1
}
