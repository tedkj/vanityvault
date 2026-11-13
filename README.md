# VanityVault

**Your own Solana vanity address, made on your own PC.**

VanityVault is a free Windows app that finds a Solana address starting or ending with a word you
choose, like one ending in `moon`. It makes the keys on your PC and keeps them in an encrypted vault
there. Your private key never passes through a website or a server, so nobody else sees it unless
you show it to them.

[Website](https://vanityvault.no) · [Download](https://github.com/tedkj/vanityvault/releases/latest)

## Download

Get the installer from the [latest release](https://github.com/tedkj/vanityvault/releases/latest).
It needs Windows 10 or 11 and installs for your Windows user only, so no admin rights are needed.

The installer is code signed. Windows shows the publisher as "Open Source Developer Ted Kenneth
Johansen" (the certificate's standard wording of the maker's full name). The certificate is new, so
Windows may say "Windows protected your PC" for a while: click "More info", check the publisher,
then "Run anyway". If it says "Unknown publisher", don't run it.

To double-check your download, run this in PowerShell. The hash should match the one on the release
page, and the signature should say `Valid`:

```powershell
Get-FileHash .\VanityVault_0.2.2-preview_x64-setup.exe
Get-AuthenticodeSignature .\VanityVault_0.2.2-preview_x64-setup.exe | Format-List
```

## How your keys stay safe

- **Made on your PC, from strong randomness.** Every key comes fresh from Windows' secure random
  source. The app never connects to Solana, never signs anything and never holds funds. Hunting
  works offline.
- **Every find is checked twice.** The graphics card finds a match, then separate code on the
  processor (the audited `ed25519-dalek` library) rebuilds the address from the key before it is
  kept. At startup the app also checks 1,024 keys from the card against the processor, and won't use
  a card that disagrees.
- **Locked in a vault.** The vault is one encrypted file on your PC, opened with your vault password
  (Argon2id with 256 MiB of memory, then XChaCha20-Poly1305). Your vault recovery key, shown once,
  is the only other way in. Neither is stored anywhere.
- **An online backup nobody else can read.** VanityCloud, the optional free backup, has its own
  VanityCloud password and VanityCloud recovery key. Each key is locked on your PC before it is
  uploaded, so the server only ever gets locked boxes and a sign-in proof that opens nothing. The app
  checks every box it gets back before trusting it.
- **You see everything that's sent.** Before the app sends anything with your data in it, it shows
  you the exact message, byte for byte, and waits for you to click Send. The website shows a real
  account exactly as the server stores it.

## What's in this repository

| Folder | What it is |
|--------|-----------|
| `app/` | The desktop app: a Rust core (`src-tauri/`) and a Svelte interface (`src/`), built with Tauri 2. |
| `app/src-tauri/kernel/` | The program the graphics card runs. `gen/make.py` rebuilds it from scratch, so you can check it comes out byte for byte the same. |
| `api/` | The VanityCloud server: a Cloudflare Worker with a D1 database. It stores locked data only. |

## Building it yourself

You need Windows, Node.js 22 or newer and a Rust toolchain.

```sh
cd app
npm ci
npm run build
cd src-tauri
cargo build --release --locked --features tauri/custom-protocol
```

The app ends up at `app/src-tauri/target/release/vanityvault.exe`. To build the installer too, run
`app\installer\build.ps1`, which also needs NSIS 3.13. Your own build isn't signed; that's expected.

The first hunt after a new graphics driver spends 2 to 4 minutes preparing the card. It hunts on the
processor meanwhile and switches by itself, and later starts are instant.

## License

[AGPL-3.0](LICENSE). Use it, change it, share it. If you hand out a changed version, or run one as
a service for other people, you share your changes under the same license.

Made in Norway by Ted K. Johansen.
