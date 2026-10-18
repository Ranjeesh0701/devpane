$ErrorActionPreference = "Stop"

$Repo = "Ranjeesh0701/devpane"
$InstallDir = "$env:LOCALAPPDATA\Programs\devpane"
$ArchiveName = "devpane-windows-x64.zip"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/$ArchiveName"

Write-Host "==> Downloading latest Devpane for Windows..." -ForegroundColor Cyan

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir | Out-Null

try {
    $ZipPath = Join-Path $TempDir $ArchiveName
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

    Write-Host "==> Extracting $ArchiveName..." -ForegroundColor Cyan
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Copy-Item (Join-Path $TempDir "devpane.exe") -Destination (Join-Path $InstallDir "devpane.exe") -Force
    Copy-Item (Join-Path $TempDir "daemon.exe") -Destination (Join-Path $InstallDir "daemon.exe") -Force

    # Add to User PATH if not already present
    $UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Host "==> Adding $InstallDir to User PATH..." -ForegroundColor Cyan
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
        $env:Path += ";$InstallDir"
    }

    Write-Host ""
    Write-Host "Devpane installed successfully!" -ForegroundColor Green
    Write-Host "   CLI:    $InstallDir\devpane.exe"
    Write-Host "   Daemon: $InstallDir\daemon.exe"
    Write-Host ""
    Write-Host "Run 'devpane up' in your project directory to get started." -ForegroundColor Yellow
}
finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
