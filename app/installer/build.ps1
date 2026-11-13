# Builds the release app and its installer: app\src-tauri\target\release\bundle\VanityVault_<version>_x64-setup.exe
#   installer\build.ps1              build the interface, the exe and the installer
#   installer\build.ps1 -SkipCargo   use an exe already built with the same RUSTFLAGS (set below)
#   installer\build.ps1 -Sign [-SignConfig <sign.json>]
# -Sign signs the exe, the uninstaller and the installer with the Certum certificate (sign.ps1;
# SimplySign Desktop connected). Off by default: without it nothing is signed. Every signature
# counts against Certum's monthly cap, so sign releases only, never dev builds.
# Windows PowerShell 5.1 or PowerShell 7.
param(
    [switch]$SkipCargo,
    [switch]$Sign,
    [string]$SignConfig = "$env:USERPROFILE\.vibeware\sign.json"
)
$ErrorActionPreference = "Stop"
if ($Sign -and -not (Test-Path -LiteralPath $SignConfig -PathType Leaf)) { throw "-Sign: no signing config at $SignConfig" }
$powershell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
$app = (Resolve-Path "$PSScriptRoot\..").Path
$tauri = "$app\src-tauri"

# makensis is the official NSIS 3.13 (nsis-3.13.zip from SourceForge, SHA-256
# ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8), unpacked to $Nsis. The whole
# folder must hash as pinned, so a changed, missing or added file stops the build. NSISDIR and
# NSISCONFDIR would make makensis read stubs or config from elsewhere, so neither may be set, and
# /NOCONFIG keeps it from reading any nsisconf.nsh.
$Nsis = if ($env:VANITYVAULT_NSIS) { $env:VANITYVAULT_NSIS } else { "C:\Projects\_tools\nsis-3.13" }
$makensis = "$Nsis\Bin\makensis.exe"
$NsisTreeSha256 = "81083599E7C4E4B30E52429E896FCC58A7F050749F6F043394843A13BBE9F927"
# SHA-256 over "relative/path<TAB>SHA-256<LF>" lines (UTF-8, upper-case hex, sorted by ordinal) of
# every file under $dir, hidden ones too.
function TreeHash([string]$dir) {
    $skip = (Get-Item -LiteralPath $dir).FullName.TrimEnd('\').Length + 1
    [string[]]$lines = @(Get-ChildItem -LiteralPath $dir -Recurse -File -Force | ForEach-Object {
        "{0}`t{1}`n" -f $_.FullName.Substring($skip).Replace('\', '/'), (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
    })
    [Array]::Sort($lines, [StringComparer]::Ordinal)
    (Get-FileHash -InputStream ([IO.MemoryStream]::new([Text.Encoding]::UTF8.GetBytes(-join $lines))) -Algorithm SHA256).Hash
}
function AssertNsis {
    foreach ($v in "NSISDIR", "NSISCONFDIR") {
        if ($null -ne [Environment]::GetEnvironmentVariable($v)) { throw "$v is set: makensis would read its stubs or config from there. Unset it." }
    }
    if (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Filter *.nsh -Force).Count -ne 0) { throw "installer\ holds an .nsh file, which would win over NSIS's own header of that name" }
    if (-not (Test-Path -LiteralPath $makensis -PathType Leaf)) { throw "no $makensis (the official NSIS 3.13; set VANITYVAULT_NSIS to its folder)" }
    $h = TreeHash $Nsis
    if ($h -ne $NsisTreeSha256) { throw "$Nsis's tree hash is $h, not the pinned ${NsisTreeSha256}: not the official NSIS 3.13" }
}
AssertNsis

$version = (Select-String -Path "$tauri\Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$parts = ($version -split '[.+-]')[0..2] | ForEach-Object { [int]$_ }
$version4 = "{0}.{1}.{2}.0" -f $parts[0], $parts[1], $parts[2]

# No user name or build path in the exe: rustc writes the source path of every panic location into
# it. Each folder is remapped to a neutral prefix (the last matching remap wins, so the profile goes
# first), and the leak check below stops the build if any such path is left.
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { "$env:USERPROFILE\.cargo" }
$rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { "$env:USERPROFILE\.rustup" }
$remaps = @("$env:USERPROFILE=/home", "$cargoHome=/cargo", "$rustupHome=/rustup", "$app=/src")
if (@($remaps | Where-Object { $_.Contains(" ") }).Count -gt 0) { throw "a build folder's path holds a space: $($remaps -join ', ')" }

if (-not $SkipCargo) {
    Push-Location $app
    try {
        npm ci
        if ($LASTEXITCODE -ne 0) { throw "npm ci failed" }
        npm run build
        if ($LASTEXITCODE -ne 0) { throw "the interface build failed" }
    } finally { Pop-Location }
    $env:RUSTFLAGS = ($remaps | ForEach-Object { "--remap-path-prefix=$_" }) -join " "
    Push-Location $tauri
    try {
        cargo build --release --locked --features tauri/custom-protocol
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally { Pop-Location }
}

$bundle = "$tauri\target\release\bundle"
New-Item -ItemType Directory -Force $bundle | Out-Null
$exe = "$bundle\VanityVault.exe"
Copy-Item "$tauri\target\release\vanityvault.exe" $exe -Force

# The leak check: the profile folder, `\<user name>\`, `C:\Users\` or `C:\Projects`, as UTF-8 or
# UTF-16, in any case. Latin-1 maps each byte to one character, so the search reads the bytes as they are.
$latin1 = [Text.Encoding]::GetEncoding(28591)
$text = $latin1.GetString([IO.File]::ReadAllBytes($exe))
$needles = @("C:\Users\", "C:\Projects", $env:USERPROFILE, "\$env:USERNAME\", "/$env:USERNAME/")
foreach ($n in $needles) {
    foreach ($enc in @([Text.Encoding]::UTF8, [Text.Encoding]::Unicode)) {
        if ($text.IndexOf($latin1.GetString($enc.GetBytes($n)), [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw "the exe holds '$n' ($($enc.WebName)): a build path or the user name. Not bundled."
        }
    }
}

# Signed after the leak check: signing adds only the signature, so the check still holds.
if ($Sign) {
    & $powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PSScriptRoot\sign.ps1" -Config $SignConfig -File $exe
    if ($LASTEXITCODE -ne 0) { throw "signing the exe failed" }
}

$out = "$bundle\VanityVault_${version}_x64-setup.exe"
# With -Sign, app.nsi's !uninstfinalize signs the uninstaller (and keeps a copy here) and its
# !finalize the installer, each through sign.ps1; without it makensis gets no SIGN define.
$uninst = "$bundle\uninstall.exe"
$signDefines = @()
if ($Sign) {
    if (Test-Path -LiteralPath $uninst) { Remove-Item -LiteralPath $uninst }
    $signDefines = @("/DSIGN=$SignConfig", "/DSIGNED_UNINSTALLER=$uninst")
}
try {
    Push-Location $PSScriptRoot
    try {
        AssertNsis
        & $makensis /NOCONFIG /V2 "/DVERSION=$version" "/DVERSION4=$version4" "/DEXE=$exe" "/DOUTFILE=$out" @signDefines app.nsi
        if ($LASTEXITCODE -ne 0) { throw "makensis failed" }
    } finally { Pop-Location }

    # All three signed by the one certificate, with a valid chain and a timestamp.
    if ($Sign) {
        $signer = $null
        foreach ($f in @($exe, $uninst, $out)) {
            if (-not (Test-Path -LiteralPath $f -PathType Leaf)) { throw "-Sign: $f is missing" }
            $s = Get-AuthenticodeSignature -LiteralPath $f
            if ($s.Status -ne "Valid") { throw "-Sign: $f's signature is $($s.Status): $($s.StatusMessage)" }
            if (-not $s.TimeStamperCertificate) { throw "-Sign: $f's signature has no timestamp" }
            if ($signer -and $s.SignerCertificate.Thumbprint -ne $signer) { throw "-Sign: $f is signed by another certificate" }
            $signer = $s.SignerCertificate.Thumbprint
            "signed  {0}  ({1})" -f (Split-Path $f -Leaf), $s.SignerCertificate.Subject.Split(',')[0]
        }
    }
} catch {
    # makensis writes the installer before !finalize signs it: a failed -Sign build leaves none.
    if ($Sign -and (Test-Path -LiteralPath $out)) { Remove-Item -LiteralPath $out }
    throw
}

$hash = (Get-FileHash -LiteralPath $out -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $(Split-Path $out -Leaf)" | Set-Content -LiteralPath "$bundle\SHA256SUMS.txt" -Encoding ascii
"{0}  {1:N0} bytes" -f $out, (Get-Item $out).Length
"sha256  $hash"
