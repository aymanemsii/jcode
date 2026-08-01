param(
    [string]$MercuryExecutablePath,
    [switch]$SkipFontInstall,
    [switch]$NoLaunch
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$StableProfileGuid = '{f97fa6de-44d7-4bdc-94a3-f8fa8228e63c}'
$FontFamily = 'Iosevka Term'
$FragmentRoot = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows Terminal\Fragments'
$FragmentDirectory = Join-Path $FragmentRoot 'Mercury'
$FragmentPath = Join-Path $FragmentDirectory 'mercury.json'
$UserFontDirectory = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Fonts'
$MercuryConfigDirectory = Join-Path $env:USERPROFILE '.mercury'
$MercuryConfigPath = Join-Path $MercuryConfigDirectory 'config.toml'
$UserFontRegistryPath = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Fonts'
$SystemFontRegistryPath = 'HKLM:\Software\Microsoft\Windows NT\CurrentVersion\Fonts'
$IosevkaReleaseApi = 'https://api.github.com/repos/be5invis/Iosevka/releases/latest'
$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path

function Write-Step {
    param([string]$Message)
    Write-Host "[setup-mercury-terminal] $Message"
}

function Fail {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Test-CommandPath {
    param([string]$CommandName)
    $cmd = Get-Command $CommandName -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $cmd) {
        return $null
    }

    $path = $cmd.Source
    if ([string]::IsNullOrWhiteSpace($path)) {
        $path = $cmd.Path
    }

    if (-not [string]::IsNullOrWhiteSpace($path) -and (Test-Path -LiteralPath $path -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $path).Path
    }

    return $null
}

function Resolve-ExistingFile {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    $expanded = [Environment]::ExpandEnvironmentVariables($Path)
    if (Test-Path -LiteralPath $expanded -PathType Leaf) {
        return (Resolve-Path -LiteralPath $expanded).Path
    }

    return $null
}

function Get-MercuryPath {
    param([string]$ExplicitPath)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        $path = Resolve-ExistingFile $ExplicitPath
        if ($null -ne $path) {
            return $path
        }

        Fail "The explicit -MercuryExecutablePath does not exist or is not a file: $ExplicitPath"
    }

    $path = Test-CommandPath 'mercury.exe'
    if ($null -ne $path) {
        return $path
    }

    $path = Test-CommandPath 'mercury'
    if ($null -ne $path) {
        return $path
    }

    foreach ($candidate in @(
        (Join-Path $env:USERPROFILE '.cargo\bin\mercury.exe'),
        (Join-Path $RepositoryRoot 'target\release\mercury.exe'),
        (Join-Path $RepositoryRoot 'target\debug\mercury.exe')
    )) {
        $path = Resolve-ExistingFile $candidate
        if ($null -ne $path) {
            return $path
        }
    }

    return $null
}

function Get-TerminalSettingsPaths {
    @(
        (Join-Path $env:LOCALAPPDATA 'Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json'),
        (Join-Path $env:LOCALAPPDATA 'Microsoft\Windows Terminal\settings.json')
    ) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
}

function Test-FontFamilyInstalled {
    param([string]$FamilyName)

    try {
        Add-Type -AssemblyName System.Drawing -ErrorAction Stop
        $families = [System.Drawing.Text.InstalledFontCollection]::new().Families
        if ($families.Name -contains $FamilyName) {
            return $true
        }
    } catch {
        Write-Step "System.Drawing font enumeration was unavailable: $($_.Exception.Message)"
    }

    foreach ($registryPath in @($UserFontRegistryPath, $SystemFontRegistryPath)) {
        if (-not (Test-Path -LiteralPath $registryPath)) {
            continue
        }

        $properties = (Get-Item -LiteralPath $registryPath).GetValueNames()
        foreach ($propertyName in $properties) {
            if ($propertyName -like "$FamilyName*") {
                return $true
            }
        }
    }

    return $false
}

function Assert-PathUnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description
    )

    $resolvedDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\')
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not ($resolvedPath -eq $resolvedDirectory -or $resolvedPath.StartsWith($resolvedDirectory + '\', [System.StringComparison]::OrdinalIgnoreCase))) {
        Fail "$Description is outside the intended current user directory. Path: $resolvedPath Intended root: $resolvedDirectory"
    }
}

function Get-UserProfileOwnerFromPath {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetPathRoot($fullPath)
    if ([string]::IsNullOrWhiteSpace($root)) {
        return $null
    }

    $withoutRoot = $fullPath.Substring($root.Length).TrimStart('\')
    $parts = $withoutRoot -split '\\'
    if ($parts.Count -ge 2 -and $parts[0] -ieq 'Users') {
        return "$root$($parts[0])\$($parts[1])"
    }

    return $null
}

function Assert-CurrentUserDestinations {
    Assert-PathUnderDirectory -Path $FragmentDirectory -Directory $env:LOCALAPPDATA -Description 'Windows Terminal fragment directory'
    Assert-PathUnderDirectory -Path $UserFontDirectory -Directory $env:LOCALAPPDATA -Description 'Per-user font directory'
    Assert-PathUnderDirectory -Path $MercuryConfigPath -Directory $env:USERPROFILE -Description 'Mercury config path'

    $folderUserProfile = [Environment]::GetFolderPath('UserProfile')
    if ((Resolve-Path -LiteralPath $env:USERPROFILE).Path -ne (Resolve-Path -LiteralPath $folderUserProfile).Path) {
        Fail "USERPROFILE and the .NET UserProfile folder disagree. USERPROFILE=$env:USERPROFILE FolderPath=$folderUserProfile"
    }

    if (-not $env:LOCALAPPDATA.StartsWith($env:USERPROFILE, [System.StringComparison]::OrdinalIgnoreCase)) {
        Fail "LOCALAPPDATA does not belong to USERPROFILE. LOCALAPPDATA=$env:LOCALAPPDATA USERPROFILE=$env:USERPROFILE"
    }
}

function Assert-RepositoryUserContext {
    param([bool]$HasExplicitMercuryPath)

    $repoOwnerProfile = Get-UserProfileOwnerFromPath $RepositoryRoot
    if ($null -eq $repoOwnerProfile) {
        return
    }

    $activeUserProfile = (Resolve-Path -LiteralPath $env:USERPROFILE).Path
    if ($repoOwnerProfile -ne $activeUserProfile -and -not $HasExplicitMercuryPath) {
        Fail "Repository is under a different Windows user profile than the active process. Repository owner: $repoOwnerProfile Active user profile: $activeUserProfile. Rerun from the intended Windows user, or pass -MercuryExecutablePath explicitly if writing the active user's Terminal/font settings is intended."
    }
}

function Get-ExistingMercuryProfiles {
    $matches = New-Object System.Collections.Generic.List[string]

    foreach ($settingsPath in Get-TerminalSettingsPaths) {
        try {
            $json = Get-Content -LiteralPath $settingsPath -Raw | ConvertFrom-Json
            foreach ($profile in @($json.profiles.list)) {
                if ($null -ne $profile -and $profile.name -eq 'Mercury') {
                    $matches.Add("settings:$settingsPath")
                }
            }
        } catch {
            Write-Step "Could not parse Windows Terminal settings at ${settingsPath}: $($_.Exception.Message)"
        }
    }

    if (Test-Path -LiteralPath $FragmentRoot) {
        Get-ChildItem -LiteralPath $FragmentRoot -Recurse -File -Filter *.json -ErrorAction SilentlyContinue | ForEach-Object {
            try {
                $json = Get-Content -LiteralPath $_.FullName -Raw | ConvertFrom-Json
                foreach ($profile in @($json.profiles)) {
                    if ($null -ne $profile -and $profile.name -eq 'Mercury') {
                        $matches.Add("fragment:$($_.FullName)")
                    }
                }
            } catch {
                Write-Step "Could not parse Windows Terminal fragment at $($_.FullName): $($_.Exception.Message)"
            }
        }
    }

    return $matches
}

function ConvertTo-WindowsTerminalCommandLine {
    param(
        [string]$ShellPath,
        [string]$MercuryPath
    )

    $escapedShell = $ShellPath.Replace('"', '\"')
    $escapedMercury = $MercuryPath.Replace("'", "''")
    return "`"$escapedShell`" -NoExit -Command `"& '$escapedMercury'`""
}

function Get-OfficialIosevkaTermAsset {
    Write-Step "Querying official Iosevka release API: $IosevkaReleaseApi"
    try {
        $release = Invoke-RestMethod -Uri $IosevkaReleaseApi -Headers @{ 'User-Agent' = 'jcode-mercury-terminal-setup' }
    } catch {
        Fail "Network access to the official Iosevka GitHub release API failed: $($_.Exception.Message)"
    }

    $assets = @($release.assets) | Where-Object {
        $_.name -match '^Pkg(TTC|TTF)-IosevkaTerm-[^/]+\.zip$'
    }

    $asset = $assets | Where-Object { $_.name -match '^PkgTTC-IosevkaTerm-' } | Select-Object -First 1
    if ($null -eq $asset) {
        $asset = $assets | Where-Object { $_.name -match '^PkgTTF-IosevkaTerm-' } | Select-Object -First 1
    }

    if ($null -eq $asset) {
        Fail "No official Iosevka Term TTC or TTF package was found in release '$($release.tag_name)'."
    }

    [PSCustomObject]@{
        Release = $release.tag_name
        Name = $asset.name
        Url = $asset.browser_download_url
    }
}

function Install-IosevkaTermFont {
    if ($SkipFontInstall) {
        Write-Step "Skipping font installation because -SkipFontInstall was provided."
        return
    }

    if (Test-FontFamilyInstalled $FontFamily) {
        Write-Step "Font family '$FontFamily' is already installed; no download needed."
        return
    }

    $asset = Get-OfficialIosevkaTermAsset
    Write-Step "Selected font package: $($asset.Name) from release $($asset.Release)"

    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("jcode-mercury-terminal-" + [System.Guid]::NewGuid().ToString('N'))
    $archivePath = Join-Path $tempRoot $asset.Name
    $extractPath = Join-Path $tempRoot 'extracted'

    New-Item -ItemType Directory -Force -Path $tempRoot, $extractPath, $UserFontDirectory | Out-Null
    Write-Step "Downloading font package to: $archivePath"

    try {
        Invoke-WebRequest -Uri $asset.Url -Headers @{ 'User-Agent' = 'jcode-mercury-terminal-setup' } -OutFile $archivePath
    } catch {
        Fail "Downloading official Iosevka Term package failed: $($_.Exception.Message)"
    }

    Write-Step "Extracting font package to: $extractPath"
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force

    $fontFiles = Get-ChildItem -LiteralPath $extractPath -Recurse -File |
        Where-Object {
            ($_.Extension -in '.ttc', '.ttf') -and
            ($_.Name -like 'IosevkaTerm-*' -or $_.Name -like 'IosevkaTerm.*')
        }

    if (@($fontFiles).Count -eq 0) {
        Fail "The official package did not contain Iosevka Term TTC/TTF font files."
    }

    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class FontNativeMethods {
    [DllImport("gdi32.dll", CharSet = CharSet.Unicode)]
    public static extern int AddFontResourceEx(string lpszFilename, uint fl, IntPtr pdv);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
}
'@

    foreach ($fontFile in $fontFiles) {
        $destination = Join-Path $UserFontDirectory $fontFile.Name
        Copy-Item -LiteralPath $fontFile.FullName -Destination $destination -Force

        $kind = if ($fontFile.Extension -ieq '.ttc') { 'TrueType Collection' } else { 'TrueType' }
        $registryName = "$FontFamily $($fontFile.BaseName) ($kind)"
        New-ItemProperty -Path $UserFontRegistryPath -Name $registryName -Value $destination -PropertyType String -Force | Out-Null
        [void][FontNativeMethods]::AddFontResourceEx($destination, 0, [IntPtr]::Zero)
        Write-Step "Installed font file: $destination"
    }

    $result = [UIntPtr]::Zero
    [void][FontNativeMethods]::SendMessageTimeout([IntPtr]0xffff, 0x001D, [UIntPtr]::Zero, 'Font', 0x0002, 5000, [ref]$result)
    Write-Step "Broadcasted WM_FONTCHANGE for the current Windows session."

    if (-not (Test-FontFamilyInstalled $FontFamily)) {
        Fail "Font installation completed but '$FontFamily' is not visible yet. Restart Windows Terminal or sign out/in, then rerun verification."
    }
}

function Write-MercuryFragment {
    param(
        [string]$MercuryPath,
        [string]$ShellPath
    )

    $startingDirectory = $RepositoryRoot

    New-Item -ItemType Directory -Force -Path $FragmentDirectory | Out-Null

    $profile = [ordered]@{
        profiles = @(
            [ordered]@{
                guid = $StableProfileGuid
                name = 'Mercury'
                tabTitle = 'Mercury'
                suppressApplicationTitle = $false
                commandline = ConvertTo-WindowsTerminalCommandLine -ShellPath $ShellPath -MercuryPath $MercuryPath
                startingDirectory = $startingDirectory
                font = [ordered]@{
                    face = $FontFamily
                    size = 13
                    weight = 'medium'
                    features = [ordered]@{
                        liga = 0
                        calt = 0
                    }
                }
            }
        )
    }

    $json = $profile | ConvertTo-Json -Depth 20
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($FragmentPath, $json + [Environment]::NewLine, $utf8NoBom)
    Write-Step "Wrote Windows Terminal fragment: $FragmentPath"
}

function Ensure-MercuryConfig {
    if (Test-Path -LiteralPath $MercuryConfigPath -PathType Leaf) {
        Write-Step "Mercury config already exists; leaving unchanged: $MercuryConfigPath"
        return
    }

    New-Item -ItemType Directory -Force -Path $MercuryConfigDirectory | Out-Null
    $content = @'
[app]
name = "Mercury"
terminal_title = "Mercury"
'@
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($MercuryConfigPath, $content + [Environment]::NewLine, $utf8NoBom)
    Write-Step "Created Mercury config: $MercuryConfigPath"
}

function Test-Fragment {
    if (-not (Test-Path -LiteralPath $FragmentPath -PathType Leaf)) {
        Fail "Expected fragment was not created: $FragmentPath"
    }

    $json = Get-Content -LiteralPath $FragmentPath -Raw | ConvertFrom-Json
    $profile = @($json.profiles) | Where-Object { $_.name -eq 'Mercury' }
    if (@($profile).Count -ne 1) {
        Fail "Expected exactly one Mercury profile in fragment, found $(@($profile).Count)."
    }
}

Write-Step "Repository path: $RepositoryRoot"
Write-Step "Active user profile: $env:USERPROFILE"
Write-Step "Active LOCALAPPDATA: $env:LOCALAPPDATA"

Assert-CurrentUserDestinations
Assert-RepositoryUserContext -HasExplicitMercuryPath (-not [string]::IsNullOrWhiteSpace($MercuryExecutablePath))

$mercuryPath = Get-MercuryPath -ExplicitPath $MercuryExecutablePath
if ($null -ne $mercuryPath) {
    Write-Step "Mercury executable: $mercuryPath"
} else {
    Write-Step "Mercury executable: not found by explicit path, Get-Command, user cargo bin, or repository target outputs"
}

$wtPath = Test-CommandPath 'wt.exe'
if ($null -ne $wtPath) {
    Write-Step "Windows Terminal executable: $wtPath"
} else {
    Write-Step "Windows Terminal executable: not found"
}

$shellPath = Test-CommandPath 'pwsh.exe'
if ($null -eq $shellPath) {
    $shellPath = Test-CommandPath 'powershell.exe'
}
if ($null -ne $shellPath) {
    Write-Step "Shell executable: $shellPath"
} else {
    Write-Step "Shell executable: neither pwsh.exe nor powershell.exe was found"
}

$existingMercuryProfiles = @(Get-ExistingMercuryProfiles)
if ($existingMercuryProfiles.Count -gt 0) {
    Write-Step "Existing Mercury profile/fragment references found:"
    $existingMercuryProfiles | ForEach-Object { Write-Step "  $_" }
} else {
    Write-Step "Existing Mercury profile/fragment references: none found"
}

$fontAlreadyInstalled = Test-FontFamilyInstalled $FontFamily
Write-Step "Font family '$FontFamily' installed: $fontAlreadyInstalled"

if ($null -eq $mercuryPath) {
    Fail "Mercury executable is required. Install Mercury or add it to PATH, then rerun this script."
}
if ($null -eq $wtPath) {
    Fail "Windows Terminal is unavailable because wt.exe was not found."
}
if ($null -eq $shellPath) {
    Fail "Neither pwsh.exe nor powershell.exe was found."
}

Install-IosevkaTermFont

if (-not (Test-FontFamilyInstalled $FontFamily)) {
    Fail "Font family '$FontFamily' is not installed or visible."
}
Write-Step "Verified font family: $FontFamily"

Ensure-MercuryConfig
Write-MercuryFragment -MercuryPath $mercuryPath -ShellPath $shellPath
Test-Fragment

$postMatches = @(Get-ExistingMercuryProfiles)
$ownFragmentMatches = @($postMatches | Where-Object { $_ -eq "fragment:$FragmentPath" })
$otherMatches = @($postMatches | Where-Object { $_ -ne "fragment:$FragmentPath" })
if ($ownFragmentMatches.Count -ne 1 -or $otherMatches.Count -gt 0) {
    Write-Step "Mercury profile references after setup:"
    $postMatches | ForEach-Object { Write-Step "  $_" }
    Fail "Duplicate or unrelated Mercury profiles/fragments were detected; only the Mercury-owned fragment should define the profile."
}

Write-Step "Verified Mercury fragment parses as JSON and contains one Mercury profile."

if ($NoLaunch) {
    Write-Step "Skipping Windows Terminal launch because -NoLaunch was provided."
} else {
    Write-Step "Launching Windows Terminal profile: wt.exe -p Mercury"
    Start-Process -FilePath $wtPath -ArgumentList @('-p', 'Mercury') -WindowStyle Hidden
}

Write-Step "Setup complete."
