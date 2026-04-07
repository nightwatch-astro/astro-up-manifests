<#
.SYNOPSIS
    Local lifecycle testing for astro-up package manifests

.DESCRIPTION
    Tests package installation, detection, and uninstallation for Windows packages.
    Downloads installers, installs silently, captures registry/PE/WMI detection info,
    and generates detection config TOML for manifests.

.PARAMETER PackageId
    Specific package ID to test (e.g., "nina-app"). If not specified, tests all packages.

.PARAMETER DryRun
    Download and probe only, skip installation and uninstallation.

.PARAMETER WhatIf
    Show what would be tested without making changes.

.EXAMPLE
    .\scripts\lifecycle-test.ps1
    Test all packages missing detection

.EXAMPLE
    .\scripts\lifecycle-test.ps1 -PackageId nina-app
    Test specific package

.EXAMPLE
    .\scripts\lifecycle-test.ps1 -DryRun
    Download and probe only
#>

[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Position = 0)]
    [string]$PackageId,

    [Parameter()]
    [switch]$DryRun,

    [Parameter()]
    [switch]$SkipUninstall,

    [Parameter()]
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

#region Initialization

# Check for Administrator privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning "This script requires Administrator privileges for installation testing."
    Write-Host "Restarting as Administrator..." -ForegroundColor Yellow

    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    if ($PackageId) { $arguments += " -PackageId '$PackageId'" }
    if ($DryRun) { $arguments += " -DryRun" }
    if ($SkipUninstall) { $arguments += " -SkipUninstall" }
    if ($Force) { $arguments += " -Force" }
    if ($WhatIfPreference) { $arguments += " -WhatIf" }

    Start-Process powershell.exe -Verb RunAs -ArgumentList $arguments
    exit
}

# Resolve script root
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not (Test-Path "$repoRoot/manifests")) {
    throw "Cannot find manifests directory at $repoRoot/manifests"
}

# Create output directories
$resultsDir = Join-Path $repoRoot "lifecycle-results"
$tempDir = Join-Path $env:TEMP "astro-up-lifecycle"
New-Item -ItemType Directory -Force -Path $resultsDir, $tempDir | Out-Null

# Logging
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARN" { "Yellow" }
        "SUCCESS" { "Green" }
        default { "White" }
    }
    Write-Host "[$timestamp] [$Level] $Message" -ForegroundColor $color
}

#endregion

#region TOML Parsing

function Get-TomlValue {
    param(
        [string]$Content,
        [string]$Key,
        [string]$Section = $null
    )

    if ($Section) {
        # Extract section
        if ($Content -match "(?ms)\[$Section\](.*?)(?=\[|$)") {
            $sectionContent = $Matches[1]
        } else {
            return $null
        }
    } else {
        $sectionContent = $Content
    }

    # Match key = "value" or key = value
    if ($sectionContent -match "$Key\s*=\s*`"([^`"]+)`"") {
        return $Matches[1]
    } elseif ($sectionContent -match "$Key\s*=\s*(\S+)") {
        return $Matches[1]
    }

    return $null
}

function Test-TomlSection {
    param([string]$Content, [string]$Section)
    return $Content -match "\[$Section\]"
}

#endregion

#region Registry Operations

function Get-UninstallRegistryKeys {
    param([string]$Filter = "*")

    $paths = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )

    $entries = @()
    foreach ($path in $paths) {
        Get-ItemProperty $path -ErrorAction SilentlyContinue | ForEach-Object {
            if ($_.PSObject.Properties['DisplayName'] -and $_.DisplayName -like $Filter) {
                $entries += $_
            }
        }
    }

    return $entries
}

function Compare-RegistrySnapshots {
    param($Before, $After, [string]$PackageName)

    $beforeKeys = $Before | ForEach-Object { $_.PSPath }
    $afterKeys = $After | ForEach-Object { $_.PSPath }

    $newKeys = $afterKeys | Where-Object { $_ -notin $beforeKeys }

    if ($newKeys) {
        Write-Log "Found $(@($newKeys).Count) new registry entries" "SUCCESS"

        foreach ($key in $newKeys) {
            $entry = $After | Where-Object { $_.PSPath -eq $key }
            if ($entry.PSObject.Properties['DisplayName'] -and $entry.DisplayName -like "*$PackageName*") {
                return $entry
            }
        }

        # Return first new entry as fallback
        return $After | Where-Object { $_.PSPath -eq $newKeys[0] }
    }

    return $null
}

#endregion

#region Version Resolution (from catalog.db)

function Initialize-Catalog {
    $catalogPath = Join-Path $repoRoot "catalog.db"
    if (-not (Test-Path $catalogPath)) {
        Write-Log "Downloading catalog.db from latest release..." "INFO"
        $releaseUrl = "https://api.github.com/repos/nightwatch-astro/astro-up-manifests/releases/tags/catalog/latest"
        try {
            $release = Invoke-RestMethod -Uri $releaseUrl -Headers @{ "User-Agent" = "astro-up-lifecycle" }
            $asset = $release.assets | Where-Object { $_.name -eq "catalog.db" }
            if ($asset) {
                Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $catalogPath -UseBasicParsing
                Write-Log "Downloaded catalog.db ($([math]::Round((Get-Item $catalogPath).Length / 1MB, 1)) MB)" "SUCCESS"
            } else {
                throw "catalog.db asset not found in release"
            }
        } catch {
            throw "Failed to download catalog.db: $_"
        }
    } else {
        Write-Log "Using existing catalog.db" "INFO"
    }
    return $catalogPath
}

function Resolve-PackageVersion {
    param(
        [hashtable]$Manifest,
        [string]$CatalogPath
    )

    $packageId = $Manifest.id
    try {
        # Use System.Data.SQLite or ADO.NET with SQLite
        Add-Type -Path "$env:ProgramFiles\System.Data.SQLite\bin\System.Data.SQLite.dll" -ErrorAction SilentlyContinue

        $conn = New-Object System.Data.SQLite.SQLiteConnection("Data Source=$CatalogPath;Read Only=True")
        $conn.Open()
        $cmd = $conn.CreateCommand()
        $cmd.CommandText = "SELECT version, url FROM versions WHERE package_id = @id AND pre_release = 0 ORDER BY discovered_at DESC LIMIT 1"
        $cmd.Parameters.AddWithValue("@id", $packageId) | Out-Null
        $reader = $cmd.ExecuteReader()
        if ($reader.Read()) {
            $version = $reader["version"]
            $url = $reader["url"]
            $reader.Close()
            $conn.Close()
            return @{ Version = $version; Url = $url }
        }
        $reader.Close()
        $conn.Close()
    } catch {
        Write-Log "SQLite query failed, trying fallback: $_" "WARN"
    }

    # Fallback: use sqlite3 CLI if available
    $sqlite3 = Get-Command sqlite3 -ErrorAction SilentlyContinue
    if ($sqlite3) {
        $result = & sqlite3 $CatalogPath "SELECT version || '|' || url FROM versions WHERE package_id = '$packageId' AND pre_release = 0 ORDER BY discovered_at DESC LIMIT 1" 2>$null
        if ($result) {
            $parts = $result -split '\|', 2
            return @{ Version = $parts[0]; Url = $parts[1] }
        }
    }

    # Fallback: use dotnet System.Data.Sqlite via Add-Type assembly load
    try {
        [System.Reflection.Assembly]::LoadFrom("$env:USERPROFILE\.nuget\packages\microsoft.data.sqlite.core\*\lib\*\Microsoft.Data.Sqlite.dll") | Out-Null
        # ... same query pattern
    } catch {}

    Write-Log "Could not resolve version for '$packageId' from catalog" "WARN"
    return $null
}

#endregion

#region Download

function Get-FileWithProgress {
    param([string]$Url, [string]$OutFile)

    try {
        Write-Log "Downloading from $Url"

        $webClient = New-Object System.Net.WebClient
        $webClient.Headers.Add("User-Agent", "astro-up-lifecycle")

        Register-ObjectEvent -InputObject $webClient -EventName DownloadProgressChanged -SourceIdentifier WebClient.DownloadProgressChanged -Action {
            Write-Progress -Activity "Downloading" -Status "$($EventArgs.ProgressPercentage)% complete" -PercentComplete $EventArgs.ProgressPercentage
        } | Out-Null

        Register-ObjectEvent -InputObject $webClient -EventName DownloadFileCompleted -SourceIdentifier WebClient.DownloadFileCompleted -Action {
            Write-Progress -Activity "Downloading" -Completed
        } | Out-Null

        $webClient.DownloadFileAsync($Url, $OutFile)

        while ($webClient.IsBusy) {
            Start-Sleep -Milliseconds 100
        }

        Unregister-Event -SourceIdentifier WebClient.DownloadProgressChanged -ErrorAction SilentlyContinue
        Unregister-Event -SourceIdentifier WebClient.DownloadFileCompleted -ErrorAction SilentlyContinue

        if (Test-Path $OutFile) {
            Write-Log "Downloaded to $OutFile" "SUCCESS"
            return $true
        }
    } catch {
        Write-Log "Download failed: $_" "ERROR"
    }

    return $false
}

#endregion

#region Installation

function Install-Package {
    param(
        [string]$InstallerPath,
        [string]$Method,
        [hashtable]$Switches
    )

    $extension = [System.IO.Path]::GetExtension($InstallerPath).ToLower()

    # Determine silent switches
    $silentArgs = if ($Switches -and $Switches.ContainsKey('silent')) {
        $Switches['silent']
    } else {
        switch ($Method) {
            "inno_setup" { "/VERYSILENT /NORESTART /SUPPRESSMSGBOXES" }
            "nullsoft" { "/S" }
            "exe" { "/S" }
            default { "" }
        }
    }

    if ($Method -eq "msi" -or $extension -eq ".msi") {
        $process = Start-Process msiexec.exe -ArgumentList "/i `"$InstallerPath`" /qn /norestart" -Wait -PassThru -NoNewWindow
    } elseif ($Method -eq "zip" -or $Method -eq "zip_wrap" -or $extension -eq ".zip") {
        $extractDir = Join-Path $tempDir "extracted"
        Expand-Archive -Path $InstallerPath -DestinationPath $extractDir -Force
        Write-Log "Extracted ZIP to $extractDir" "SUCCESS"
        return @{ Success = $true; ExitCode = 0; Message = "ZIP extracted" }
    } else {
        $process = Start-Process -FilePath $InstallerPath -ArgumentList $silentArgs -Wait -PassThru -NoNewWindow
    }

    $timeout = 300 # 5 minutes
    $waited = 0
    while (-not $process.HasExited -and $waited -lt $timeout) {
        Start-Sleep -Seconds 1
        $waited++
    }

    if (-not $process.HasExited) {
        Write-Log "Installation timeout after $timeout seconds, killing process" "WARN"
        $process.Kill()
        return @{ Success = $false; ExitCode = -1; Message = "Timeout" }
    }

    $exitCode = $process.ExitCode
    $success = $exitCode -eq 0 -or $exitCode -eq 3010 # 3010 = reboot required

    return @{
        Success = $success
        ExitCode = $exitCode
        Message = if ($success) { "Installed successfully" } else { "Installation failed with exit code $exitCode" }
    }
}

#endregion

#region Detection

function Get-PEVersionInfo {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        return $null
    }

    $exeFiles = Get-ChildItem -Path $Path -Filter "*.exe" -Recurse -ErrorAction SilentlyContinue |
        Where-Object { -not $_.PSIsContainer } |
        Select-Object -First 10

    $results = @()
    foreach ($exe in $exeFiles) {
        try {
            $versionInfo = $exe.VersionInfo
            if ($versionInfo.FileVersion -or $versionInfo.ProductVersion) {
                $results += @{
                    Path = $exe.FullName
                    FileVersion = $versionInfo.FileVersion
                    ProductVersion = $versionInfo.ProductVersion
                    ProductName = $versionInfo.ProductName
                    CompanyName = $versionInfo.CompanyName
                }
            }
        } catch {
            # Skip files we can't read
        }
    }

    return $results
}

function Get-WMIProduct {
    param([string]$PackageName)

    try {
        Write-Log "Querying WMI (this may take a while)..." "INFO"
        $products = Get-CimInstance -ClassName Win32_Product -Filter "Name LIKE '%$PackageName%'" -ErrorAction SilentlyContinue
        return $products | Select-Object Name, Version, IdentifyingNumber
    } catch {
        Write-Log "WMI query failed: $_" "WARN"
        return $null
    }
}

function New-DetectionConfig {
    param([hashtable]$DetectionInfo)

    $lines = @()
    $lines += "[detection]"

    if ($DetectionInfo.Method -eq "registry") {
        $lines += "method = `"registry`""
        $lines += "registry_key = `"$($DetectionInfo.RegistryKey)`""
        if ($DetectionInfo.RegistryValue) {
            $lines += "registry_value = `"$($DetectionInfo.RegistryValue)`""
        }
    } elseif ($DetectionInfo.Method -eq "pe_file") {
        $lines += "method = `"pe_file`""
        $lines += "path = `"$($DetectionInfo.Path)`""
    }

    return $lines -join "`n"
}

#endregion

#region Uninstallation

function Uninstall-Package {
    param([object]$RegistryEntry)

    if (-not $RegistryEntry) {
        Write-Log "No registry entry provided for uninstallation" "WARN"
        return $false
    }

    # Try QuietUninstallString first, then UninstallString
    $uninstallCmd = if ($RegistryEntry.PSObject.Properties['QuietUninstallString']) { $RegistryEntry.QuietUninstallString } else { $null }
    if (-not $uninstallCmd -and $RegistryEntry.PSObject.Properties['UninstallString']) {
        $uninstallCmd = $RegistryEntry.UninstallString
    }

    if (-not $uninstallCmd) {
        Write-Log "No uninstall command found" "WARN"
        return $false
    }

    Write-Log "Uninstalling with: $uninstallCmd"

    try {
        # Parse command and arguments
        if ($uninstallCmd -match '^"([^"]+)"(.*)') {
            $exe = $Matches[1]
            $args = $Matches[2].Trim()
        } elseif ($uninstallCmd -match '^(\S+)(.*)') {
            $exe = $Matches[1]
            $args = $Matches[2].Trim()
        } else {
            $exe = $uninstallCmd
            $args = ""
        }

        # Add silent flags if not present
        if ($args -notmatch '/S|/VERYSILENT|/qn') {
            if ($exe -like "*msiexec*") {
                $args += " /qn /norestart"
            } elseif ($uninstallCmd -like "*Inno*" -or $uninstallCmd -like "*Setup*") {
                $args += " /VERYSILENT /NORESTART"
            } else {
                $args += " /S"
            }
        }

        $process = Start-Process -FilePath $exe -ArgumentList $args -Wait -PassThru -NoNewWindow

        $success = $process.ExitCode -eq 0 -or $process.ExitCode -eq 3010
        if ($success) {
            Write-Log "Uninstallation successful" "SUCCESS"
        } else {
            Write-Log "Uninstallation failed with exit code $($process.ExitCode)" "WARN"
        }

        return $success
    } catch {
        Write-Log "Uninstallation error: $_" "ERROR"
        return $false
    }
}

#endregion

#region Main Testing Logic

function Test-PackageLifecycle {
    param([hashtable]$Manifest, [string]$ManifestPath)

    $packageId = $Manifest.id
    $packageName = $Manifest.name

    Write-Log "========================================" "INFO"
    Write-Log "Testing: $packageId ($packageName)" "INFO"
    Write-Log "========================================" "INFO"

    $result = @{
        PackageId = $packageId
        PackageName = $packageName
        Version = $null
        Download = $null
        Install = $null
        Detection = $null
        Uninstall = $null
        DetectionConfig = $null
        Error = $null
    }

    try {
        # 1. Resolve version from catalog
        Write-Log "Step 1: Resolving version from catalog"
        $versionInfo = Resolve-PackageVersion -Manifest $Manifest -CatalogPath $script:catalogPath
        if (-not $versionInfo) {
            throw "Could not resolve version from catalog for '$packageId'"
        }
        $version = $versionInfo.Version
        $result.Version = $version
        Write-Log "Resolved version: $version" "SUCCESS"

        # 2. Download installer
        Write-Log "Step 2: Downloading installer"
        $autoupdateUrl = if ($Manifest.ContainsKey('autoupdate_url')) { $Manifest.autoupdate_url } else { $null }
        $downloadUrl = if ($versionInfo.Url) { $versionInfo.Url } elseif ($autoupdateUrl) { $autoupdateUrl -replace '\$version', $version } else { throw "No download URL available" }
        $installerFileName = [System.IO.Path]::GetFileName($downloadUrl)
        if ($installerFileName -notmatch '\.\w+$') {
            $installerFileName = "$packageId-$version.exe"
        }
        $installerPath = Join-Path $tempDir $installerFileName

        if (-not (Get-FileWithProgress -Url $downloadUrl -OutFile $installerPath)) {
            throw "Download failed"
        }
        $result.Download = "OK"

        if ($DryRun) {
            Write-Log "DryRun mode: skipping installation" "INFO"
            $result.Install = "SKIPPED"
            $result.Detection = "SKIPPED"
            $result.Uninstall = "SKIPPED"
            return $result
        }

        # 3. Pre-install snapshot
        Write-Log "Step 3: Capturing pre-install state"
        $beforeRegistry = Get-UninstallRegistryKeys

        # 4. Install
        Write-Log "Step 4: Installing package"
        $switches = if ($Manifest.ContainsKey('install_switches')) { $Manifest.install_switches } else { @{} }
        $installResult = Install-Package -InstallerPath $installerPath -Method $Manifest.install_method -Switches $switches

        if (-not $installResult.Success) {
            throw "Installation failed: $($installResult.Message)"
        }
        $result.Install = "OK (exit code: $($installResult.ExitCode))"
        Write-Log $installResult.Message "SUCCESS"

        # Wait a moment for registry to settle
        Start-Sleep -Seconds 2

        # 5. Post-install snapshot
        Write-Log "Step 5: Capturing post-install state"
        $afterRegistry = Get-UninstallRegistryKeys

        # 6. Detection probes
        Write-Log "Step 6: Running detection probes"
        $newEntry = Compare-RegistrySnapshots -Before $beforeRegistry -After $afterRegistry -PackageName $packageName

        $detectionInfo = @{}

        if ($newEntry) {
            $displayName = if ($newEntry.PSObject.Properties['DisplayName']) { $newEntry.DisplayName } else { "unknown" }
            $displayVersion = if ($newEntry.PSObject.Properties['DisplayVersion']) { $newEntry.DisplayVersion } else { "" }
            $publisher = if ($newEntry.PSObject.Properties['Publisher']) { $newEntry.Publisher } else { "" }
            $installLocation = if ($newEntry.PSObject.Properties['InstallLocation']) { $newEntry.InstallLocation } else { "" }

            Write-Log "Found registry entry: $displayName" "SUCCESS"
            Write-Log "  Version: $displayVersion"
            Write-Log "  Publisher: $publisher"
            Write-Log "  Install Location: $installLocation"

            # Extract registry key
            $regPath = $newEntry.PSPath -replace 'Microsoft\.PowerShell\.Core\\Registry::', ''
            $regKey = $regPath -replace '\\DisplayName$', ''

            $detectionInfo = @{
                Method = "registry"
                RegistryKey = $regKey
                RegistryValue = "DisplayVersion"
                Name = $displayName
                Version = $displayVersion
                InstallLocation = $installLocation
            }

            # PE scan if we have install location
            if ($installLocation) {
                Write-Log "Scanning for PE files..."
                $peInfo = Get-PEVersionInfo -Path $installLocation
                if ($peInfo) {
                    Write-Log "Found $(@($peInfo).Count) executables with version info" "SUCCESS"
                    $detectionInfo.PEFiles = $peInfo
                }
            }

            $result.Detection = "OK (registry)"
            $result.DetectionConfig = New-DetectionConfig -DetectionInfo $detectionInfo
        } else {
            Write-Log "No registry entry found" "WARN"
            $result.Detection = "FAILED (no registry entry)"
        }

        # 7. Uninstall
        if (-not $SkipUninstall -and $newEntry) {
            Write-Log "Step 7: Uninstalling package"
            $uninstallSuccess = Uninstall-Package -RegistryEntry $newEntry
            $result.Uninstall = if ($uninstallSuccess) { "OK" } else { "FAILED" }

            # Verify removal
            Start-Sleep -Seconds 2
            $verifyRegistry = Get-UninstallRegistryKeys -Filter "*$packageName*"
            if (-not $verifyRegistry) {
                Write-Log "Verified: package removed from registry" "SUCCESS"
            } else {
                Write-Log "Warning: package still present in registry" "WARN"
            }
        } else {
            $result.Uninstall = "SKIPPED"
        }

    } catch {
        $result.Error = $_.Exception.Message
        Write-Log "Test failed: $($_.Exception.Message)" "ERROR"
    }

    # Save results
    $resultFile = Join-Path $resultsDir "$packageId.json"
    $result | ConvertTo-Json -Depth 10 | Set-Content -Path $resultFile
    Write-Log "Results saved to $resultFile"

    # Append detection config to manifest if found
    if ($result.DetectionConfig -and $PSCmdlet.ShouldProcess($ManifestPath, "Add detection config")) {
        Write-Log "Appending detection config to manifest"
        Add-Content -Path $ManifestPath -Value "`n$($result.DetectionConfig)"
    }

    return $result
}

#endregion

#region Package Selection

function Get-PackagesToTest {
    $manifestFiles = Get-ChildItem -Path "$repoRoot/manifests" -Filter "*.toml"
    $packages = @()

    foreach ($file in $manifestFiles) {
        $content = Get-Content $file.FullName -Raw

        # Skip if already has detection (unless -Force)
        if (-not $Force -and (Test-TomlSection -Content $content -Section "detection")) {
            continue
        }

        # Skip resource packages
        $type = Get-TomlValue -Content $content -Key "type"
        if ($type -eq "resource") {
            continue
        }

        # Parse manifest
        $manifest = @{
            id = Get-TomlValue -Content $content -Key "id"
            name = Get-TomlValue -Content $content -Key "name"
            type = $type
            install_method = Get-TomlValue -Content $content -Key "method" -Section "install"
            checkver_provider = Get-TomlValue -Content $content -Key "provider" -Section "checkver"
            checkver_owner = Get-TomlValue -Content $content -Key "owner" -Section "checkver"
            checkver_repo = Get-TomlValue -Content $content -Key "repo" -Section "checkver"
            checkver_url = Get-TomlValue -Content $content -Key "url" -Section "checkver"
            checkver_regex = Get-TomlValue -Content $content -Key "regex" -Section "checkver"
            autoupdate_url = Get-TomlValue -Content $content -Key "url" -Section "checkver.autoupdate"
        }

        # Parse install switches if present
        $silentSwitch = Get-TomlValue -Content $content -Key "silent" -Section "install.switches"
        if ($silentSwitch) {
            $manifest.install_switches = @{ silent = $silentSwitch }
        }

        $packages += @{
            Manifest = $manifest
            ManifestPath = $file.FullName
        }
    }

    return $packages
}

#endregion

#region Main Execution

# Get packages to test
$packagesToTest = Get-PackagesToTest

if ($PackageId) {
    $packagesToTest = $packagesToTest | Where-Object { $_.Manifest.id -eq $PackageId }
    if (-not $packagesToTest) {
        Write-Log "Package '$PackageId' not found or already has detection config" "ERROR"
        exit 1
    }
}

# Initialize catalog for version resolution
$script:catalogPath = Initialize-Catalog

Write-Log "Found $(@($packagesToTest).Count) packages to test"

$allResults = @()

foreach ($pkg in $packagesToTest) {
    $result = Test-PackageLifecycle -Manifest $pkg.Manifest -ManifestPath $pkg.ManifestPath
    $allResults += $result
}

# Summary table
Write-Log "`n========================================" "INFO"
Write-Log "SUMMARY" "INFO"
Write-Log "========================================" "INFO"

$summaryTable = $allResults | ForEach-Object {
    [PSCustomObject]@{
        Package = $_.PackageId
        Version = $_.Version
        Install = $_.Install
        Detect = $_.Detection
        Uninstall = $_.Uninstall
        Error = if ($_.Error) { $_.Error.Substring(0, [Math]::Min(40, $_.Error.Length)) } else { "" }
    }
}

$summaryTable | Format-Table -AutoSize

# Offer to commit changes
$updatedManifests = $allResults | Where-Object { $_.DetectionConfig } | ForEach-Object { $_.PackageId }

if ($updatedManifests -and -not $WhatIfPreference) {
    Write-Log "`nUpdated $(@($updatedManifests).Count) manifests with detection config" "SUCCESS"
    Write-Log "Packages: $($updatedManifests -join ', ')"

    $commit = Read-Host "`nCommit changes to git? (y/N)"
    if ($commit -eq 'y') {
        Push-Location $repoRoot
        try {
            git add manifests/
            $commitMsg = "feat: add detection config for $($updatedManifests -join ', ')"
            git commit -m $commitMsg
            Write-Log "Committed changes" "SUCCESS"

            $push = Read-Host "Push to remote? (y/N)"
            if ($push -eq 'y') {
                git push
                Write-Log "Pushed to remote" "SUCCESS"
            }
        } finally {
            Pop-Location
        }
    }
}

Write-Log "`nLifecycle testing complete!" "SUCCESS"
Write-Log "Results saved to: $resultsDir"

#endregion
