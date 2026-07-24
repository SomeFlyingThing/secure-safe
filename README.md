# secure_safe

`secure_safe` is a small command-line vault for locally storing encrypted files. It compresses a file, encrypts it with a key derived from your password, and saves the encrypted entry in a private vault directory.

## How it works

- Files are compressed with Zstandard before storage.
- Encryption uses XChaCha20-Poly1305, which also detects tampering and incorrect passwords.
- Encryption keys are derived from the password using Argon2 and a fresh random salt for every file.
- The original path is kept inside the encrypted entry so a restored file returns to its original location.

On first use, the application creates:

- `~/.safe_dir/` — the default directory containing encrypted files.
- `~/secure_safe.settings` — a TOML settings file that can point `enc_dir` at a different vault location.

## Install

You need a current Rust toolchain. Build an optimized binary with:

```sh
cargo build --release
```

The binary will be available at `target/release/secure_safe`.

## Usage

```text
secure_safe <COMMAND> [PATH]
```

| Command | Description |
| --- | --- |
| `add <PATH>` | Encrypt and store a file. |
| `rm <NAME>` | Permanently remove an encrypted entry from the vault. |
| `mo <NAME>` | Decrypt, restore, and remove an encrypted entry from the vault. |
| `check` | Verify every stored entry using the supplied password. |
| `help` | Display command help. |

Long forms are also supported: `--add`, `--rm`, `--mo`, `--check`, and `--help`.

Examples:

```sh
secure_safe add secret.txt
secure_safe check
secure_safe mo secret.txt
secure_safe rm secret.txt
```

`rm` and `mo` take the stored file name, not a path. Stored names are the original file's base name. For example, `secure_safe add documents/secret.txt` creates the entry `secret.txt`.

## Important notes

- Keep your password safe. There is no password recovery mechanism.
- Adding a file does **not** delete the original plaintext file. Delete it yourself only after confirming the encrypted entry can be restored.
- Restoring an entry writes to its original path and then removes the encrypted vault entry. If a file already exists at the original path, restoration fails rather than overwriting it.
- Entries with the same base filename conflict in the vault, even when their original paths differ.
- `check` confirms that entries can be authenticated with the entered password; it prints each verified stored filename.

## Configure the vault directory

Edit `~/secure_safe.settings` to set another directory:

```toml
enc_dir = "/absolute/path/to/my-vault"
```

The directory is created automatically when the program runs.

## Development

Run the test suite with:

```sh
cargo test
```
