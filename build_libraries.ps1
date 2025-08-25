# parse the command line arguments
param(
    [Parameter(Position=0)]
    [ValidateSet("release", "debug", "both")]
    [string]$BuildType = "both"
)

# build the main project
Write-Host "building the main project"
if ($BuildType -eq "release" -or $BuildType -eq "both") {
    cargo build --release
} else {
    cargo build
}

# build all the library projects
$libraries = @(
    "library_io",
    "library_common",
    "library_example",
    "library_os",
    "library_time",
    "library_http",
    "library_fs",
    "library_json",
    "library_math"
)

# create the target directory for release
if ($BuildType -eq "release" -or $BuildType -eq "both") {
    # create the target directory for release
    $targetDir = ".\target\release\library"
    if (-not (Test-Path $targetDir)) {
        Write-Host "create: $targetDir"
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
}

if ($BuildType -eq "debug" -or $BuildType -eq "both") {
    # create the target directory for debug
    $debugTargetDir = ".\target\debug\library"
    if (-not (Test-Path $debugTargetDir)) {
        Write-Host "create: $debugTargetDir"
        New-Item -ItemType Directory -Path $debugTargetDir -Force | Out-Null
    }
}

foreach ($lib in $libraries) {
    Write-Host "building: $lib"
    cd .\$lib
    
    # build the library
    if ($BuildType -eq "release") {
        cargo build --release
    } elseif ($BuildType -eq "debug") {
        cargo build
    } else {
        # both - build debug first, then build release
        cargo build
        cargo build --release
    }
    
    # get the output file name
    $libName = $lib -replace "library_", ""
    
    # release version
    if ($BuildType -eq "release" -or $BuildType -eq "both") {
        $sourceFile = ".\target\release\$libName.dll"
        $targetFile = "..\target\release\library\$libName.dll"
        
        # if the file exists, copy it
        if (Test-Path $sourceFile) {
            Write-Host "copy: $sourceFile -> $targetFile"
            Copy-Item -Path $sourceFile -Destination $targetFile -Force
        } else {
            Write-Host "warning: $sourceFile not found, skip copy"
        }
    }
    
    # debug version
    if ($BuildType -eq "debug" -or $BuildType -eq "both") {
        $sourceFile = ".\target\debug\$libName.dll"
        $targetFile = "..\target\debug\library\$libName.dll"
        
        # if the file exists, copy it
        if (Test-Path $sourceFile) {
            Write-Host "copy: $sourceFile -> $targetFile"
            Copy-Item -Path $sourceFile -Destination $targetFile -Force
        } else {
            Write-Host "warning: $sourceFile not found, skip copy"
        }
    }
    
    # return to the root directory
    cd ..
}

Write-Host "all libraries done"