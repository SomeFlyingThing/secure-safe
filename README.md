<div align="center">

# 🔐 secure_safe

### A small, local command-line vault for files that should stay yours.

<p>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&logoColor=white">
  <img alt="Encryption" src="https://img.shields.io/badge/Encryption-XChaCha20--Poly1305-6E56CF?logo=letsencrypt&logoColor=white">
  <img alt="Key derivation" src="https://img.shields.io/badge/Key%20derivation-Argon2-1F8AC0">
  <img alt="Storage" src="https://img.shields.io/badge/Storage-local%20only-2EA44F">
</p>

<p><code>secure_safe</code> compresses and encrypts files locally, then keeps them in a private vault directory.</p>

</div>

<br>

## ✨ What it does

| | |
| :-- | :-- |
| 🗜️ **Compresses first** | Reduces file size with Zstandard before storage. |
| 🔒 **Encrypts locally** | Uses authenticated XChaCha20-Poly1305 encryption. |
| 🧂 **Derives strong keys** | Uses Argon2 with a fresh random salt for every file. |
| 📍 **Remembers where files came from** | Restores each file to its original path. |
| 🛡️ **Checks integrity** | Detects tampering and incorrect passwords during verification. |

> [!IMPORTANT]
> This is a local vault, not a backup service. Keep a safe copy of anything you cannot afford to lose.

## 🚀 Install

You need a current [Rust toolchain](https://www.rust-lang.org/tools/install).

```sh
cargo build --release
```

Your optimized binary will be at:

```text
target/release/secure_safe
```

## ⚡ Quick start

```sh
# Encrypt and store a file
secure_safe add secret.txt

# Verify every vault entry with your password
secure_safe check

# Restore a file to its original path, then remove its vault entry
secure_safe mo secret.txt
```

## 🧰 Commands

```text
secure_safe <COMMAND> [PATH]
```

<table>
  <thead>
    <tr><th align="left">Command</th><th align="left">What it does</th></tr>
  </thead>
  <tbody>
    <tr><td><code>add &lt;PATH&gt;</code></td><td>Encrypt and store a file.</td></tr>
    <tr><td><code>rm &lt;NAME&gt;</code></td><td>Permanently remove an encrypted vault entry.</td></tr>
    <tr><td><code>mo &lt;NAME&gt;</code></td><td>Decrypt to the original path, then remove the vault entry.</td></tr>
    <tr><td><code>check</code></td><td>Verify every stored entry with the supplied password.</td></tr>
    <tr><td><code>help</code></td><td>Show command help.</td></tr>
  </tbody>
</table>

Long forms are available too: `--add`, `--rm`, `--mo`, `--check`, and `--help`.

### Example session

```sh
secure_safe add documents/secret.txt
secure_safe check
secure_safe mo secret.txt
secure_safe rm secret.txt
```

<details>
  <summary><strong>About stored names</strong></summary>
  <br>
  Vault entries use the source file's base name. For example, adding
  <code>documents/secret.txt</code> stores an entry named <code>secret.txt</code>.
  Files with the same base name conflict, even if they came from different directories.
</details>

## 🗂️ Vault location

On first use, `secure_safe` creates:

```text
~/.safe_dir/              # encrypted files
~/secure_safe.settings    # TOML configuration
```

Set another vault location by editing `~/secure_safe.settings`:

```toml
enc_dir = "/absolute/path/to/my-vault"
```

The directory is created automatically when the program runs.

## ⚠️ Before you trust it with a file

> [!WARNING]
> There is no password recovery. If you lose your password, the encrypted data cannot be recovered.

- `add` **does not delete** the original plaintext file. Remove it yourself only after you have confirmed that restoration works.
- `mo` restores the file to its original location, then removes the encrypted vault entry.
- Restoration refuses to overwrite an existing file at the original path.
- `check` authenticates every entry using the entered password and prints each verified stored filename.

## 🛠️ Development

```sh
cargo test
```

<div align="center">
  <sub>Keep the key. Keep control.</sub>
</div>
