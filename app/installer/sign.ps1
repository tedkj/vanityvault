# Signs one file for a release with the Certum code signing certificate (SimplySign in the cloud).
# build.ps1 -Sign runs it on the app exe; makensis runs it (app.nsi's !uninstfinalize and
# !finalize) on the uninstaller before it is packed and on the finished installer.
#   sign.ps1 -Config <sign.json> -File <file> [-Keep <path>]
# -Keep: a copy of the signed file there (the uninstaller, for build.ps1's check).
# Exit 0 once the file carries a timestamped signature by the configured certificate, 1 (with the
# reason on stderr) when it does not.
#
# The config lives outside the repo (%USERPROFILE%\.vibeware\sign.json) and holds no secret:
#   signtool    signtool.exe (Windows SDK)
#   thumbprint  the SHA-1 thumbprint of the code signing certificate in Cert:\CurrentUser\My;
#               SimplySign Desktop puts it there while it is connected, the key stays in the cloud
#   timestamp   the RFC 3161 timestamp server; left out: http://time.certum.pl
# Digests are SHA-256 (file and timestamp). Windows PowerShell 5.1.
param(
    [Parameter(Mandatory = $true)][string]$Config,
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Keep = ""
)
$ErrorActionPreference = "Stop"
# Always its own Windows PowerShell process. Started from under PowerShell 7 (makensis under a pwsh
# build.ps1), it inherits pwsh's PSModulePath, whose modules it can't load: the built-in ones only.
$env:PSModulePath = "$PSHOME\Modules"

function Fail([string]$why) {
    [Console]::Error.WriteLine("sign.ps1: $why")
    exit 1
}

function Field($cfg, [string]$name) {
    $p = $cfg.PSObject.Properties[$name]
    if ($p) { return $p.Value }
    return $null
}

try {
    if (-not (Test-Path -LiteralPath $Config -PathType Leaf)) { Fail "no signing config at $Config" }
    if (-not (Test-Path -LiteralPath $File -PathType Leaf)) { Fail "no file to sign at $File" }
    $cfg = Get-Content -LiteralPath $Config -Raw | ConvertFrom-Json

    $signtool = Field $cfg "signtool"
    if (-not ($signtool -is [string]) -or -not (Test-Path -LiteralPath $signtool -PathType Leaf)) { Fail "$Config's `"signtool`" is not a file: $signtool" }
    $thumbprint = Field $cfg "thumbprint"
    if (-not ($thumbprint -is [string]) -or $thumbprint -notmatch '^[0-9A-Fa-f]{40}$') { Fail "$Config has no `"thumbprint`" (40 hex digits)" }
    $thumbprint = $thumbprint.ToUpperInvariant()
    $timestamp = Field $cfg "timestamp"
    if ($null -eq $timestamp) { $timestamp = "http://time.certum.pl" }
    # The certificate lives a year: a signature without a timestamp dies with it.
    if (-not ($timestamp -is [string]) -or $timestamp -notmatch '^https?://[^\s"]+$') { Fail "$Config's `"timestamp`" is not an http(s) URL: $timestamp" }

    $cert = Get-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -ErrorAction SilentlyContinue
    if (-not $cert) { Fail "certificate $thumbprint is not in Cert:\CurrentUser\My: is SimplySign Desktop connected?" }
    if (-not $cert.HasPrivateKey) { Fail "certificate $thumbprint has no private key here: is SimplySign Desktop connected?" }
    if ($cert.NotAfter -lt (Get-Date)) { Fail "certificate $thumbprint expired $($cert.NotAfter)" }

    & $signtool sign /fd SHA256 /tr $timestamp /td SHA256 /s My /sha1 $thumbprint $File
    if ($LASTEXITCODE -ne 0) { Fail "signtool failed ($LASTEXITCODE) on $File" }

    $s = Get-AuthenticodeSignature -LiteralPath $File
    if (-not $s.SignerCertificate) { Fail "$File carries no signature after signtool ($($s.Status))" }
    if ($s.SignerCertificate.Thumbprint -ne $thumbprint) { Fail "$File is signed by $($s.SignerCertificate.Thumbprint), not $thumbprint" }
    if (-not $s.TimeStamperCertificate) { Fail "$File's signature has no timestamp" }
    if ($Keep -ne "") { Copy-Item -LiteralPath $File -Destination $Keep -Force }
} catch {
    Fail "$_"
}
exit 0
