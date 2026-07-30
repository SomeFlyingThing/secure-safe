<div align="center">
  <h1>🔐 secure_safe</h1>
  <p><strong>An experimental local encrypted-file vault written in Rust.</strong></p>
  <p>
    <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&amp;logoColor=white">
    <img alt="XChaCha20-Poly1305" src="https://img.shields.io/badge/encryption-XChaCha20--Poly1305-6E56CF">
    <img alt="Argon2" src="https://img.shields.io/badge/key%20derivation-Argon2-1F8AC0">
    <img alt="Status: experimental" src="https://img.shields.io/badge/status-experimental-D73A49">
  </p>
</div>

<hr>

<blockquote>
  <p><strong>Do not use this version as the only copy of important files.</strong> The project is unaudited and pre-alpha. Writes are not yet atomic or crash-safe, and some error paths still panic.</p>
</blockquote>

<h2>What it does</h2>

<p><code>secure_safe</code> compresses a file, encrypts it into a local vault, and removes the original. A stored file can later be authenticated, decrypted, decompressed, and restored to its original path.</p>

<table>
  <tbody>
    <tr>
      <td>🗜️ <strong>Compression</strong></td>
      <td>Zstandard level 5 before encryption.</td>
    </tr>
    <tr>
      <td>🧂 <strong>Key derivation</strong></td>
      <td>Argon2 with a fresh random 16-byte salt for each file.</td>
    </tr>
    <tr>
      <td>🔒 <strong>Encryption</strong></td>
      <td>Authenticated XChaCha20-Poly1305 with a fresh 24-byte nonce.</td>
    </tr>
    <tr>
      <td>📍 <strong>Path binding</strong></td>
      <td>The original path is stored and authenticated as associated data.</td>
    </tr>
    <tr>
      <td>🧹 <strong>Secret handling</strong></td>
      <td>Entered passwords and derived keys use zeroizing wrappers.</td>
    </tr>
  </tbody>
</table>

<p>These are implementation details, not a security guarantee.</p>

<h2>Build</h2>

<p>Install a current Rust toolchain, clone the repository, and run:</p>

<pre><code>cargo build --release
./target/release/secure_safe help</code></pre>

<p>For development:</p>

<pre><code>cargo build
cargo test</code></pre>

<h2>Usage</h2>

<pre><code>secure_safe &lt;COMMAND&gt; [PATH]</code></pre>

<table>
  <thead>
    <tr>
      <th align="left">Command</th>
      <th align="left">Behavior</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>add &lt;PATH&gt;</code></td>
      <td>Compresses and encrypts a file into the vault, then removes the source file.</td>
    </tr>
    <tr>
      <td><code>rm &lt;NAME&gt;</code></td>
      <td>After confirmation, overwrites the named vault entry with zero bytes and removes it.</td>
    </tr>
    <tr>
      <td><code>mo &lt;NAME&gt;</code></td>
      <td>Decrypts and restores the named vault entry to its recorded path, then removes the vault entry.</td>
    </tr>
    <tr>
      <td><code>check</code></td>
      <td>Attempts to authenticate every stored entry with the supplied password.</td>
    </tr>
    <tr>
      <td><code>help</code></td>
      <td>Prints command help.</td>
    </tr>
  </tbody>
</table>

<p>Long forms are also accepted: <code>--add</code>, <code>--rm</code>, <code>--mo</code>, <code>--check</code>, and <code>--help</code>.</p>

<h3>Examples</h3>

<pre><code># Encrypt a file and remove the plaintext source
secure_safe add ~/Documents/secret.txt

# Restore it to ~/Documents/secret.txt
secure_safe mo secret.txt

# Check all vault entries with a password
secure_safe check

# Permanently remove an entry without restoring it
secure_safe rm secret.txt</code></pre>

<h2>Built-in file explorer</h2>

<p>Run <code>secure_safe add</code> without a path to select a source file interactively. The explorer starts in your home directory and groups directories before files.</p>

<table>
  <thead>
    <tr>
      <th align="left">Key</th>
      <th align="left">Action</th>
    </tr>
  </thead>
  <tbody>
    <tr><td><code>↑</code> / <code>↓</code></td><td>Move the selection.</td></tr>
    <tr><td><code>→</code></td><td>Open the selected directory.</td></tr>
    <tr><td><code>←</code></td><td>Go to the parent directory.</td></tr>
    <tr><td><code>Enter</code></td><td>Choose the selected file.</td></tr>
    <tr><td><code>q</code> / <code>Esc</code></td><td>Quit.</td></tr>
  </tbody>
</table>

<p>For <code>rm</code> and <code>mo</code>, pass the vault entry's base name explicitly.</p>

<h2>Storage</h2>

<p>On first use, the program creates:</p>

<table>
  <thead>
    <tr>
      <th align="left">Path</th>
      <th align="left">Purpose</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>~/.safe_dir/</code></td>
      <td>Default encrypted-entry directory.</td>
    </tr>
    <tr>
      <td><code>~/secure_safe.settings</code></td>
      <td>TOML settings file containing <code>enc_dir</code>.</td>
    </tr>
  </tbody>
</table>

<p>To use a different vault directory, edit the settings file:</p>

<pre><code>enc_dir = "/absolute/path/to/my-vault"</code></pre>

<h2>Known limitations</h2>

<ul>
  <li>There is no password recovery. Each file is decrypted with the password used when it was added.</li>
  <li>Vault writes and source deletion are not atomic or explicitly synced. A crash or power loss at the wrong time can cause data loss.</li>
  <li>Several I/O and parsing paths still use <code>unwrap</code>, <code>expect</code>, or <code>panic</code>.</li>
  <li>Vault entries use only the source base filename, so files with the same name conflict even when they come from different directories.</li>
  <li>Restoring can replace a file already present at the recorded path.</li>
  <li>Overwriting before unlinking does not guarantee physical erasure on SSDs, copy-on-write filesystems, snapshots, journals, or remote storage.</li>
  <li>The on-disk format is not versioned and may change without migration support.</li>
  <li>The implementation has not received an independent security audit.</li>
</ul>

<h2>Before production use</h2>

<ol>
  <li>Make vault writes and source deletion durable, atomic, and recoverable.</li>
  <li>Replace panic-based error handling across file and cryptographic operations.</li>
  <li>Prevent accidental overwrite when restoring and define conflict behavior.</li>
  <li>Version and document the on-disk format.</li>
  <li>Add broader failure-path and interruption tests.</li>
  <li>Obtain a focused independent security review.</li>
</ol>

<div align="center">
  <sub>Experimental cryptography code: inspect first, trust later.</sub>
</div>
