$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ("seamlesscontrol-update-test-" + [guid]::NewGuid().ToString('N'))
try {
    $old = Join-Path $root 'installed'
    $new = Join-Path $root 'release'
    New-Item -ItemType Directory -Force -Path $old, $new | Out-Null
    $names = @('seamlesscontrol.exe', 'seamlesscontrold.exe')
    foreach ($name in $names) {
        Set-Content -LiteralPath (Join-Path $old $name) -Value "old $name"
        Set-Content -LiteralPath (Join-Path $new $name) -Value "new $name"
    }
    $archive = Join-Path $root 'seamlesscontrol-windows-x64.zip'
    Compress-Archive -Path (Join-Path $new '*.exe') -DestinationPath $archive
    $digest = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    "$digest  seamlesscontrol-windows-x64.zip" | Set-Content -Encoding ascii -LiteralPath "$archive.sha256"
    $result = Join-Path $root 'result.txt'
    $script = Join-Path $PSScriptRoot '../packaging/windows-update.ps1'

    & $script -Archive $archive -InstallDir $old -ParentPid 999999 -ResultFile $result -NoRestart
    if ((Get-Content $result -Raw) -notmatch '^Updated app and agent together') { throw 'successful update was not reported' }
    foreach ($name in $names) {
        if ((Get-Content (Join-Path $old $name) -Raw) -notmatch '^new ') { throw "failed to replace $name" }
    }

    Set-Content -Encoding ascii -LiteralPath "$archive.sha256" -Value ('0' * 64 + '  seamlesscontrol-windows-x64.zip')
    & $script -Archive $archive -InstallDir $old -ParentPid 999999 -ResultFile $result -NoRestart
    if ((Get-Content $result -Raw) -notmatch '^Update failed: The ZIP does not match') { throw 'checksum mismatch was not rejected' }
    foreach ($name in $names) {
        if ((Get-Content (Join-Path $old $name) -Raw) -notmatch '^new ') { throw "mismatch changed $name" }
    }
    Write-Host 'Windows bundled update passed.'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
