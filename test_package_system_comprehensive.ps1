# CodeNothing Package System Comprehensive Test Script
# Tests all package system features including imports, exports, and dependencies

param(
    [switch]$Verbose = $false,
    [switch]$StopOnError = $false
)

# Test configuration
$TestResults = @()
$TotalTests = 0
$PassedTests = 0
$FailedTests = 0

# Colors for output
$Green = "Green"
$Red = "Red"
$Yellow = "Yellow"
$Cyan = "Cyan"

function Write-TestHeader {
    param([string]$Title)
    Write-Host "`n=== $Title ===" -ForegroundColor $Cyan
}

function Write-TestResult {
    param(
        [string]$TestName,
        [bool]$Passed,
        [string]$Details = ""
    )
    
    $global:TotalTests++
    
    if ($Passed) {
        $global:PassedTests++
        Write-Host "✓ $TestName" -ForegroundColor $Green
        if ($Verbose -and $Details) {
            Write-Host "  $Details" -ForegroundColor Gray
        }
    } else {
        $global:FailedTests++
        Write-Host "✗ $TestName" -ForegroundColor $Red
        if ($Details) {
            Write-Host "  $Details" -ForegroundColor $Red
        }
        
        if ($StopOnError) {
            Write-Host "Stopping on first error as requested." -ForegroundColor $Red
            exit 1
        }
    }
    
    $global:TestResults += @{
        Name = $TestName
        Passed = $Passed
        Details = $Details
    }
}

function Test-Compilation {
    param(
        [string]$TestName,
        [string]$SourceFile,
        [bool]$ShouldSucceed = $true
    )
    
    try {
        $output = & cargo run -- $SourceFile 2>&1
        $exitCode = $LASTEXITCODE
        
        if ($ShouldSucceed) {
            $success = ($exitCode -eq 0)
            $details = if ($success) { "Compilation successful" } else { "Exit code: $exitCode" }
        } else {
            $success = ($exitCode -ne 0)
            $details = if ($success) { "Expected compilation failure" } else { "Unexpected compilation success" }
        }
        
        Write-TestResult -TestName $TestName -Passed $success -Details $details
        
        if ($Verbose) {
            Write-Host "Output: $output" -ForegroundColor Gray
        }
        
        return $success
    }
    catch {
        Write-TestResult -TestName $TestName -Passed $false -Details "Exception: $($_.Exception.Message)"
        return $false
    }
}

function Test-Execution {
    param(
        [string]$TestName,
        [string]$SourceFile,
        [string]$ExpectedOutput = $null
    )
    
    try {
        $output = & cargo run -- $SourceFile 2>&1
        $exitCode = $LASTEXITCODE
        
        $success = ($exitCode -eq 0)
        
        if ($success -and $ExpectedOutput) {
            $success = $output -match [regex]::Escape($ExpectedOutput)
            $details = if ($success) { "Output contains expected text" } else { "Expected output not found" }
        } else {
            $details = if ($success) { "Execution successful" } else { "Exit code: $exitCode" }
        }
        
        Write-TestResult -TestName $TestName -Passed $success -Details $details
        
        if ($Verbose) {
            Write-Host "Output: $output" -ForegroundColor Gray
        }
        
        return $success
    }
    catch {
        Write-TestResult -TestName $TestName -Passed $false -Details "Exception: $($_.Exception.Message)"
        return $false
    }
}

# Main test execution
Write-Host "CodeNothing Package System Comprehensive Test Suite" -ForegroundColor $Cyan
Write-Host "=================================================" -ForegroundColor $Cyan

# Test 1: Basic Package Declaration
Write-TestHeader "Basic Package System Tests"
Test-Compilation -TestName "Math Package Compilation" -SourceFile "test_package_math.cn"
Test-Compilation -TestName "Utils Package Compilation" -SourceFile "test_package_utils.cn"

# Test 2: Package Dependencies
Write-TestHeader "Package Dependency Tests"
Test-Compilation -TestName "Main Package with Dependencies" -SourceFile "test_package_main.cn"
Test-Compilation -TestName "Selective Import Package" -SourceFile "test_package_selective.cn"

# Test 3: Package Execution Tests
Write-TestHeader "Package Execution Tests"
if (Test-Compilation -TestName "Math Package Build Check" -SourceFile "test_package_math.cn") {
    Test-Execution -TestName "Math Package Execution" -SourceFile "test_package_math.cn"
}

if (Test-Compilation -TestName "Utils Package Build Check" -SourceFile "test_package_utils.cn") {
    Test-Execution -TestName "Utils Package Execution" -SourceFile "test_package_utils.cn"
}

# Test 4: Integration Tests
Write-TestHeader "Package Integration Tests"
if (Test-Compilation -TestName "Main Program Build Check" -SourceFile "test_package_main.cn") {
    Test-Execution -TestName "Main Program with Package Imports" -SourceFile "test_package_main.cn" -ExpectedOutput "Sum:"
}

if (Test-Compilation -TestName "Selective Import Build Check" -SourceFile "test_package_selective.cn") {
    Test-Execution -TestName "Selective Import Execution" -SourceFile "test_package_selective.cn" -ExpectedOutput "Result:"
}

# Test 5: Error Handling Tests
Write-TestHeader "Package Error Handling Tests"

# Create a test file with invalid package syntax
$invalidPackageContent = @"
package invalid-name version "1.0.0";
import nonexistent_package;
export fn test() : int { return 42; }
"@
$invalidPackageContent | Out-File -FilePath "test_invalid_package.cn" -Encoding UTF8

Test-Compilation -TestName "Invalid Package Syntax" -SourceFile "test_invalid_package.cn" -ShouldSucceed $false

# Clean up test file
Remove-Item "test_invalid_package.cn" -ErrorAction SilentlyContinue

# Test 6: Compatibility Tests
Write-TestHeader "Backward Compatibility Tests"

# Create a mixed include/package test
$mixedContent = @"
include "wildcard_utils_a.cn";
package mixed version "1.0.0";
import math { add };

fn test_mixed() {
    var result1 = func_a(5);
    var result2 = add(10, 20);
    print("Mixed result: " + (result1 + result2));
}
"@
$mixedContent | Out-File -FilePath "test_mixed_system.cn" -Encoding UTF8

Test-Compilation -TestName "Mixed Include/Package System" -SourceFile "test_mixed_system.cn"

# Clean up
Remove-Item "test_mixed_system.cn" -ErrorAction SilentlyContinue

# Test Summary
Write-TestHeader "Test Summary"
Write-Host "Total Tests: $TotalTests" -ForegroundColor $Cyan
Write-Host "Passed: $PassedTests" -ForegroundColor $Green
Write-Host "Failed: $FailedTests" -ForegroundColor $(if ($FailedTests -eq 0) { $Green } else { $Red })

$successRate = if ($TotalTests -gt 0) { [math]::Round(($PassedTests / $TotalTests) * 100, 2) } else { 0 }
Write-Host "Success Rate: $successRate%" -ForegroundColor $(if ($successRate -ge 90) { $Green } elseif ($successRate -ge 70) { $Yellow } else { $Red })

# Detailed results if verbose
if ($Verbose) {
    Write-TestHeader "Detailed Results"
    foreach ($result in $TestResults) {
        $status = if ($result.Passed) { "PASS" } else { "FAIL" }
        $color = if ($result.Passed) { $Green } else { $Red }
        Write-Host "[$status] $($result.Name)" -ForegroundColor $color
        if ($result.Details) {
            Write-Host "       $($result.Details)" -ForegroundColor Gray
        }
    }
}

# Exit with appropriate code
if ($FailedTests -eq 0) {
    Write-Host "`nAll tests passed! Package system is working correctly." -ForegroundColor $Green
    exit 0
} else {
    Write-Host "`nSome tests failed. Please check the implementation." -ForegroundColor $Red
    exit 1
}
