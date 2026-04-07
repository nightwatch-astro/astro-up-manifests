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
    [switch]$Force,

    [Parameter()]
    [switch]$AutoCommit
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

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
    if ($AutoCommit) { $arguments += " -AutoCommit" }
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

#region TOML Parsing (PSToml)

# Ensure PSToml is installed
if (-not (Get-Module -ListAvailable -Name PSToml)) {
    Write-Log "Installing PSToml module..." "INFO"
    Install-Module PSToml -Scope CurrentUser -Force -AllowClobber
}
Import-Module PSToml -ErrorAction Stop

function Read-ManifestToml {
    param([string]$Path)
    $content = Get-Content $Path -Raw
    return $content | ConvertFrom-Toml
}

function Test-TomlSection {
    param([object]$Toml, [string]$Section)
    return $null -ne $Toml.$Section
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

    $newKeys = @($afterKeys | Where-Object { $_ -notin $beforeKeys })

    if ($newKeys.Count -gt 0) {
        Write-Log "Found $($newKeys.Count) new registry entries" "SUCCESS"

        # Try name match first
        foreach ($key in $newKeys) {
            $entry = $After | Where-Object { $_.PSPath -eq $key }
            if ($entry.PSObject.Properties['DisplayName'] -and $entry.DisplayName -like "*$PackageName*") {
                Write-Log "  Matched by name: $($entry.DisplayName)"
                return $entry
            }
        }

        # No name match — return first new entry (it's what we just installed)
        $fallback = $After | Where-Object { $_.PSPath -eq $newKeys[0] }
        $fbName = if ($fallback.PSObject.Properties['DisplayName']) { $fallback.DisplayName } else { "unknown" }
        Write-Log "  No name match, using first new entry: $fbName" "WARN"
        return $fallback
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

        # Use curl.exe — PowerShell's Invoke-WebRequest has TLS issues with many vendor sites
        $curlExe = Get-Command curl.exe -ErrorAction SilentlyContinue
        if ($curlExe) {
            & curl.exe -L -o $OutFile --fail --silent --show-error --progress-bar $Url 2>&1 | ForEach-Object { Write-Host $_ }
        } else {
            # Fallback to Invoke-WebRequest
            Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
        }

        if ((Test-Path $OutFile) -and (Get-Item $OutFile).Length -gt 0) {
            $sizeMB = [math]::Round((Get-Item $OutFile).Length / 1MB, 1)
            Write-Log "Downloaded to $OutFile ($sizeMB MB)" "SUCCESS"
            return $true
        } else {
            Write-Log "Download produced empty file" "ERROR"
            return $false
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

    Write-Log "  Install method: '$Method', Extension: '$extension'" "INFO"

    # Determine silent switches
    # For explicit methods (inno_setup, nullsoft, msi), use manifest switches.
    # For generic "exe", always auto-detect from binary — manifest switches are often wrong.
    $silentArgs = if ($Method -ne "exe" -and $Switches -and $Switches.Contains('silent')) {
        $Switches['silent']
    } else {
        # For generic "exe", detect installer type from binary
        $effectiveMethod = $Method
        if ($Method -eq "exe" -or -not $Method) {
            $bytes = [System.IO.File]::ReadAllBytes($InstallerPath)
            $text = [System.Text.Encoding]::ASCII.GetString($bytes, 0, [Math]::Min(65536, $bytes.Length))
            if ($text -match "Inno Setup") {
                $effectiveMethod = "inno_setup"
                Write-Log "  Detected InnoSetup installer" "INFO"
            } elseif ($text -match "Nullsoft") {
                $effectiveMethod = "nullsoft"
                Write-Log "  Detected Nullsoft/NSIS installer" "INFO"
            } elseif ($text -match "WiX" -or $text -match "Windows Installer XML") {
                $effectiveMethod = "burn"
                Write-Log "  Detected WiX/Burn installer" "INFO"
            }
        }
        switch ($effectiveMethod) {
            "inno_setup" { "/VERYSILENT /NORESTART /SUPPRESSMSGBOXES" }
            "nullsoft" { "/S" }
            "burn" { "/quiet /norestart" }
            "exe" { "/S" }
            default { "" }
        }
    }
    Write-Log "  Silent args: '$silentArgs'" "INFO"

    if ($Method -eq "msi" -or $extension -eq ".msi") {
        $process = Start-Process msiexec.exe -ArgumentList "/i `"$InstallerPath`" /qn /norestart" -Wait -PassThru -NoNewWindow
    } elseif ($Method -eq "zip" -or $Method -eq "zip_wrap" -or $extension -eq ".zip") {
        $extractDir = Join-Path $tempDir "extracted"
        Expand-Archive -Path $InstallerPath -DestinationPath $extractDir -Force
        Write-Log "Extracted ZIP to $extractDir" "SUCCESS"
        return @{ Success = $true; ExitCode = 0; Message = "ZIP extracted" }
    } else {
        if ($silentArgs) {
            $process = Start-Process -FilePath $InstallerPath -ArgumentList $silentArgs -Wait -PassThru -NoNewWindow
        } else {
            $process = Start-Process -FilePath $InstallerPath -Wait -PassThru -NoNewWindow
        }
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

function Get-WMISnapshot {
    param([string]$PackageName, [string[]]$Aliases = @())

    $snapshot = @{ Products = @(); Drivers = @() }
    $searchTerms = @($PackageName) + $Aliases | Where-Object { $_ }

    # Win32_InstalledWin32Program (fast, no MSI enumeration)
    try {
        Write-Log "  WMI: querying installed programs..." "INFO"
        foreach ($term in $searchTerms) {
            $products = Get-CimInstance -ClassName Win32_InstalledWin32Program -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -like "*$term*" }
            if ($products) {
                $snapshot.Products += @($products | Select-Object Name, Version, ProgramId)
            }
        }
    } catch {
        Write-Log "  WMI Win32_InstalledWin32Program failed: $_" "WARN"
    }

    # Win32_PnPSignedDriver (for device drivers)
    try {
        Write-Log "  WMI: querying signed drivers..." "INFO"
        foreach ($term in $searchTerms) {
            $drivers = Get-CimInstance -ClassName Win32_PnPSignedDriver -ErrorAction SilentlyContinue |
                Where-Object { $_.DriverProviderName -like "*$term*" -or $_.DeviceName -like "*$term*" } |
                Select-Object DeviceName, DriverProviderName, DriverVersion, DeviceClass, InfName -First 5
            if ($drivers) {
                $snapshot.Drivers += @($drivers)
            }
        }
    } catch {
        Write-Log "  WMI driver query failed: $_" "WARN"
    }

    return $snapshot
}

function Find-InstalledFiles {
    param([string]$PackageName, [string]$InstallLocation)

    $results = @()
    $searchDirs = @()

    # Use install location if known
    if ($InstallLocation -and (Test-Path $InstallLocation)) {
        $searchDirs += $InstallLocation
    }

    # Also search common program directories
    $programDirs = @(
        "$env:ProgramFiles",
        "${env:ProgramFiles(x86)}",
        "$env:LOCALAPPDATA\Programs"
    )
    foreach ($dir in $programDirs) {
        if (Test-Path $dir) {
            $match = Get-ChildItem -Path $dir -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -like "*$PackageName*" } | Select-Object -First 3
            if ($match) { $searchDirs += @($match.FullName) }
        }
    }

    foreach ($dir in ($searchDirs | Select-Object -Unique)) {
        $exes = Get-ChildItem -Path $dir -Filter "*.exe" -ErrorAction SilentlyContinue |
            Where-Object { -not $_.PSIsContainer } | Select-Object -First 5
        foreach ($exe in $exes) {
            try {
                $vi = $exe.VersionInfo
                $results += @{
                    Path = $exe.FullName
                    FileVersion = $vi.FileVersion
                    ProductVersion = $vi.ProductVersion
                    ProductName = $vi.ProductName
                    CompanyName = $vi.CompanyName
                }
            } catch {}
        }
    }

    return $results
}

function New-DetectionConfig {
    param([hashtable]$DetectionInfo)

    $lines = @()
    $lines += "[detection]"

    if ($DetectionInfo.Method -eq "registry") {
        $escapedKey = $DetectionInfo.RegistryKey .Replace('\', '\\')
        $lines += "method = `"registry`""
        $lines += "registry_key = `"$escapedKey`""
        if ($DetectionInfo.RegistryValue) {
            $lines += "registry_value = `"$($DetectionInfo.RegistryValue)`""
        }
    } elseif ($DetectionInfo.Method -eq "pe_file") {
        $escapedPath = $DetectionInfo.Path .Replace('\', '\\')
        $lines += "method = `"pe_file`""
        $lines += "path = `"$escapedPath`""
    } elseif ($DetectionInfo.Method -eq "wmi") {
        $lines += "method = `"wmi`""
        if ($DetectionInfo.DriverProvider) {
            $lines += "inf_provider = `"$($DetectionInfo.DriverProvider)`""
        }
        if ($DetectionInfo.DeviceClass) {
            $lines += "device_class = `"$($DetectionInfo.DeviceClass)`""
        }
        if ($DetectionInfo.InfName) {
            $lines += "inf_name = `"$($DetectionInfo.InfName)`""
        }
    } elseif ($DetectionInfo.Method -eq "file") {
        $escapedPath = $DetectionInfo.Path .Replace('\', '\\')
        $lines += "method = `"file`""
        $lines += "path = `"$escapedPath`""
    }

    # Add fallback if available
    if ($DetectionInfo.Fallback) {
        $fb = $DetectionInfo.Fallback
        $lines += ""
        $lines += "[detection.fallback]"
        $lines += "method = `"$($fb.Method)`""
        if ($fb.Path) {
            $escapedFb = $fb.Path .Replace('\', '\\')
            $lines += "path = `"$escapedFb`""
        }
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
        $autoupdateUrl = if ($Manifest.Contains('autoupdate_url')) { $Manifest.autoupdate_url } else { $null }
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
        $switches = if ($Manifest.Contains('install_switches')) { $Manifest.install_switches } else { @{} }
        $installResult = Install-Package -InstallerPath $installerPath -Method $Manifest.install_method -Switches $switches

        if (-not $installResult.Success) {
            throw "Installation failed: $($installResult.Message)"
        }
        $result.Install = "OK (exit code: $($installResult.ExitCode))"
        Write-Log $installResult.Message "SUCCESS"

        # Wait a moment for registry to settle
        Start-Sleep -Seconds 2

        # 5. Post-install registry snapshot
        Write-Log "Step 5: Capturing post-install state"
        $afterRegistry = Get-UninstallRegistryKeys

        # 6. WMI snapshot
        Write-Log "Step 6: WMI snapshot"
        $wmiSnapshot = Get-WMISnapshot -PackageName $packageName
        if ($wmiSnapshot.Products.Count -gt 0) {
            Write-Log "  WMI found $($wmiSnapshot.Products.Count) matching products" "SUCCESS"
            foreach ($p in $wmiSnapshot.Products) {
                Write-Log "    $($p.Name) v$($p.Version)" "INFO"
            }
        }
        if ($wmiSnapshot.Drivers.Count -gt 0) {
            Write-Log "  WMI found $($wmiSnapshot.Drivers.Count) matching drivers" "SUCCESS"
            foreach ($d in $wmiSnapshot.Drivers) {
                Write-Log "    $($d.DeviceName) [$($d.DriverProviderName)] v$($d.DriverVersion) inf=$($d.InfName)" "INFO"
            }
        }

        # 7. File search
        Write-Log "Step 7: File search"
        $fileResults = Find-InstalledFiles -PackageName $packageName -InstallLocation ""
        if (@($fileResults).Count -gt 0) {
            Write-Log "  Found $(@($fileResults).Count) executables" "SUCCESS"
            foreach ($f in $fileResults) {
                Write-Log "    $($f.Path) v$($f.ProductVersion) [$($f.ProductName)]" "INFO"
            }
        }

        # 8. Registry diff detection
        Write-Log "Step 8: Registry diff detection"
        $newEntry = Compare-RegistrySnapshots -Before $beforeRegistry -After $afterRegistry -PackageName $packageName

        # Build detection info — try each method, pick best
        $detectionInfo = @{}
        $detectionMethod = $null

        # Method 1: Registry (highest confidence if found)
        if ($newEntry) {
            $displayName = if ($newEntry.PSObject.Properties['DisplayName']) { $newEntry.DisplayName } else { "unknown" }
            $displayVersion = if ($newEntry.PSObject.Properties['DisplayVersion']) { $newEntry.DisplayVersion } else { "" }
            $publisher = if ($newEntry.PSObject.Properties['Publisher']) { $newEntry.Publisher } else { "" }
            $installLocation = if ($newEntry.PSObject.Properties['InstallLocation']) { $newEntry.InstallLocation } else { "" }

            Write-Log "Registry: $displayName v$displayVersion" "SUCCESS"
            Write-Log "  Publisher: $publisher"
            Write-Log "  Install Location: $installLocation"

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
            $detectionMethod = "registry"

            # Enrich: PE scan for fallback
            $scanLocation = if ($installLocation) { $installLocation } else { $null }
            if ($scanLocation) {
                $peInfo = Get-PEVersionInfo -Path $scanLocation
                if ($peInfo) {
                    Write-Log "  PE fallback: $(@($peInfo).Count) executables with version info" "SUCCESS"
                    $detectionInfo.PEFiles = $peInfo
                    # Add PE as fallback detection
                    $bestPE = @($peInfo) | Where-Object { $_.ProductVersion } | Select-Object -First 1
                    if ($bestPE) {
                        $detectionInfo.Fallback = @{
                            Method = "pe_file"
                            Path = $bestPE.Path
                        }
                    }
                }
            }
        }

        # Method 2: WMI driver (if no registry match, check drivers)
        if (-not $detectionMethod -and $wmiSnapshot.Drivers.Count -gt 0) {
            $bestDriver = $wmiSnapshot.Drivers | Select-Object -First 1
            Write-Log "WMI driver: $($bestDriver.DeviceName) v$($bestDriver.DriverVersion)" "SUCCESS"
            $detectionInfo = @{
                Method = "wmi"
                DriverProvider = $bestDriver.DriverProviderName
                DeviceClass = $bestDriver.DeviceClass
                InfName = $bestDriver.InfName
                Version = $bestDriver.DriverVersion
                Name = $bestDriver.DeviceName
            }
            $detectionMethod = "wmi"
        }

        # Method 3: PE file (if no registry or WMI match)
        if (-not $detectionMethod -and @($fileResults).Count -gt 0) {
            $bestPE = @($fileResults) | Where-Object { $_.ProductVersion } | Select-Object -First 1
            if ($bestPE) {
                Write-Log "PE file: $($bestPE.Path) v$($bestPE.ProductVersion)" "SUCCESS"
                $detectionInfo = @{
                    Method = "pe_file"
                    Path = $bestPE.Path
                    Version = $bestPE.ProductVersion
                    Name = $bestPE.ProductName
                }
                $detectionMethod = "pe_file"
            }
        }

        # Method 4: File exists (last resort — just check exe exists)
        if (-not $detectionMethod -and @($fileResults).Count -gt 0) {
            $bestFile = @($fileResults) | Select-Object -First 1
            Write-Log "File exists: $($bestFile.Path)" "SUCCESS"
            $detectionInfo = @{
                Method = "file"
                Path = $bestFile.Path
                Name = $bestFile.ProductName
            }
            $detectionMethod = "file"
        }

        # Generate config
        if ($detectionMethod) {
            $result.Detection = "OK ($detectionMethod)"
            $result.DetectionConfig = New-DetectionConfig -DetectionInfo $detectionInfo
            Write-Log "Best detection method: $detectionMethod" "SUCCESS"
        } else {
            Write-Log "No detection method found" "WARN"
            $result.Detection = "FAILED (no detection)"
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

    # Write detection config to manifest (replace existing or append)
    if ($result.DetectionConfig -and $PSCmdlet.ShouldProcess($ManifestPath, "Add detection config")) {
        $content = Get-Content -Path $ManifestPath -Raw

        # Remove existing [detection] and [detection.*] sections (line-by-line to handle sub-sections)
        $lines = $content -split "`n"
        $filtered = @()
        $inDetection = $false
        foreach ($line in $lines) {
            if ($line -match '^\[detection') {
                $inDetection = $true
                continue
            }
            if ($inDetection -and $line -match '^\[' -and $line -notmatch '^\[detection') {
                $inDetection = $false
            }
            if (-not $inDetection) {
                $filtered += $line
            }
        }
        $content = ($filtered -join "`n").TrimEnd()

        # Append new detection config
        $content += "`n`n$($result.DetectionConfig)`n"
        Set-Content -Path $ManifestPath -Value $content -NoNewline
        Write-Log "Wrote detection config to manifest" "SUCCESS"
    }

    return $result
}

#endregion

#region Package Selection

function Get-PackagesToTest {
    $manifestFiles = Get-ChildItem -Path "$repoRoot/manifests" -Filter "*.toml"
    $packages = @()

    foreach ($file in $manifestFiles) {
        $toml = Read-ManifestToml -Path $file.FullName

        # Skip if already has detection (unless -Force)
        if (-not $Force -and (Test-TomlSection -Toml $toml -Section "detection")) {
            continue
        }

        # Skip resource packages
        if ($toml.type -eq "resource") {
            continue
        }

        # Build manifest hashtable from parsed TOML
        $install = if ($toml.Contains('install')) { $toml.install } else { @{} }
        $checkver = if ($toml.Contains('checkver')) { $toml.checkver } else { @{} }
        $autoupdate = if ($checkver.Contains('autoupdate')) { $checkver.autoupdate } else { @{} }
        $switches = if ($install.Contains('switches')) { $install.switches } else { @{} }

        $manifest = @{
            id = $toml.id
            name = $toml.name
            type = $toml.type
            install_method = if ($install.Contains('method')) { $install.method } else { "" }
            install_switches = $switches
            autoupdate_url = if ($autoupdate.Contains('url')) { $autoupdate.url } else { $null }
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
        Error = if ($_.Error) { $_.Error } else { "" }
    }
}

$summaryTable | Format-Table -AutoSize

# Offer to commit changes
$updatedManifests = $allResults | Where-Object { $_.DetectionConfig } | ForEach-Object { $_.PackageId }

if ($updatedManifests -and -not $WhatIfPreference) {
    Write-Log "`nUpdated $(@($updatedManifests).Count) manifests with detection config" "SUCCESS"
    Write-Log "Packages: $($updatedManifests -join ', ')"

    $commit = if ($AutoCommit) { 'y' } else { Read-Host "`nCommit changes to git? (y/N)" }
    if ($commit -eq 'y') {
        Push-Location $repoRoot
        try {
            git add manifests/
            $commitMsg = "feat: add detection config for $($updatedManifests -join ', ')"
            git commit -m $commitMsg
            Write-Log "Committed changes" "SUCCESS"

            $push = if ($AutoCommit) { 'y' } else { Read-Host "Push to remote? (y/N)" }
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
