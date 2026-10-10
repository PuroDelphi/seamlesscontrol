param(
    [Parameter(Mandatory = $true)][string]$Archive,
    [Parameter(Mandatory = $true)][string]$InstallDir,
    [Parameter(Mandatory = $true)][int]$ParentPid,
    [Parameter(Mandatory = $true)][string]$ResultFile,
    [switch]$NoRestart
)

$ErrorActionPreference = 'Stop'
$names = @('seamlesscontrol.exe', 'seamlesscontrold.exe')
$stage = Join-Path ([IO.Path]::GetTempPath()) ("seamlesscontrol-update-" + [guid]::NewGuid().ToString('N'))
$backups = @()
$installed = @()

try {
    if ([IO.Path]::GetFileName($Archive) -cne 'seamlesscontrol-windows-x64.zip') {
        throw 'Choose the official seamlesscontrol-windows-x64.zip release asset.'
    }
    $sidecar = "$Archive.sha256"
    if (-not (Test-Path -LiteralPath $sidecar -PathType Leaf)) {
        throw 'Download the matching .zip.sha256 asset into the same folder first.'
    }
    $line = (Get-Content -LiteralPath $sidecar -Raw).Trim()
    if ($line -cnotmatch '^([0-9a-f]{64})  seamlesscontrol-windows-x64\.zip$') {
        throw 'The checksum file has an unexpected format.'
    }
    $actual = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne $Matches[1]) { throw 'The ZIP does not match its SHA256 file.' }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $entries = @($zip.Entries)
        if ($entries.Count -ne 2) { throw 'The ZIP must contain exactly the app and agent.' }
        foreach ($name in $names) {
            $found = @($entries | Where-Object { $_.FullName -ceq $name -and $_.Length -gt 0 -and $_.Length -le 104857600 })
            if ($found.Count -ne 1) { throw "Missing or invalid $name in the ZIP." }
        }
        New-Item -ItemType Directory -Force -Path $stage | Out-Null
        foreach ($entry in $entries) {
            $target = Join-Path $stage $entry.FullName
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target, $false)
        }
    } finally { $zip.Dispose() }

    $deadline = (Get-Date).AddSeconds(30)
    while (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) {
        if ((Get-Date) -gt $deadline) { throw 'The running app did not close for the update.' }
        Start-Sleep -Milliseconds 200
    }
    foreach ($name in $names) {
        $target = Join-Path $InstallDir $name
        $backup = "$target.seamlesscontrol-backup"
        if (-not (Test-Path -LiteralPath $target -PathType Leaf)) { throw "Installed $name is missing." }
        if (Test-Path -LiteralPath $backup) { throw "Old backup exists for $name; remove it after checking the previous update." }
        Move-Item -LiteralPath $target -Destination $backup -ErrorAction Stop
        $backups += @{ Target = $target; Backup = $backup }
        Move-Item -LiteralPath (Join-Path $stage $name) -Destination $target -ErrorAction Stop
        $installed += $target
    }
    foreach ($item in $backups) { Remove-Item -LiteralPath $item.Backup -Force -ErrorAction SilentlyContinue }
    Set-Content -LiteralPath $ResultFile -Encoding UTF8 -Value 'Updated app and agent together. Preferences and pairing are preserved.'
} catch {
    foreach ($target in $installed) { Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue }
    foreach ($item in $backups) {
        if (Test-Path -LiteralPath $item.Backup) {
            Move-Item -LiteralPath $item.Backup -Destination $item.Target -Force -ErrorAction SilentlyContinue
        }
    }
    Set-Content -LiteralPath $ResultFile -Encoding UTF8 -Value ("Update failed: " + $_.Exception.Message)
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    $app = Join-Path $InstallDir 'seamlesscontrol.exe'
    if (-not $NoRestart -and (Test-Path -LiteralPath $app -PathType Leaf)) { Start-Process -FilePath $app }
}
