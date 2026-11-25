# Installation

`mdfm` is distributed as a prebuilt Rust binary via Homebrew, APT, Scoop, and GitHub Releases, or can be compiled from source with Cargo.

## :beer: Homebrew (macOS / Linux)

Install `mdfm` via [Homebrew](https://brew.sh/) using the [`nicholaswilde/homebrew-tap`](https://github.com/nicholaswilde/homebrew-tap):

```bash
brew install nicholaswilde/homebrew-tap/mdfm
```

Or tap the repository first:

```bash
brew tap nicholaswilde/homebrew-tap
brew install mdfm
```

## :package: APT (Debian / Ubuntu)

Install `.deb` packages (`amd64`, `arm64`, `armhf`, `armel`) from the [`nicholaswilde/apt`](https://github.com/nicholaswilde/apt) repository ([nicholaswilde.io/apt](https://nicholaswilde.io/apt)), which bundles the binary, man page, and Bash/Zsh/Fish shell completions:

### 1. Add Repository Signing Key

```bash
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://nicholaswilde.io/apt/public.key | sudo gpg --dearmor -o /etc/apt/keyrings/nicholaswilde-apt.gpg
sudo chmod a+r /etc/apt/keyrings/nicholaswilde-apt.gpg
```

### 2. Add APT Source

=== "Deb822 (Debian 12+ / Ubuntu 22.04+)"

    ```bash
    sudo tee /etc/apt/sources.list.d/nicholaswilde.sources << 'EOF'
    Types: deb
    URIs: https://nicholaswilde.io/apt
    Suites: stable
    Components: main
    Signed-By: /etc/apt/keyrings/nicholaswilde-apt.gpg
    EOF
    ```

=== "Traditional One-Line (`.list`)"

    ```bash
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/nicholaswilde-apt.gpg] https://nicholaswilde.io/apt stable main" | sudo tee /etc/apt/sources.list.d/nicholaswilde-apt.list
    ```

### 3. Update & Install

```bash
sudo apt update
sudo apt install mdfm
```

## :icecream: Scoop (Windows)

Install `mdfm` via [Scoop](https://scoop.sh/) using [`nicholaswilde/scoop-bucket`](https://github.com/nicholaswilde/scoop-bucket):

```powershell
scoop bucket add nicholaswilde https://github.com/nicholaswilde/scoop-bucket
scoop install nicholaswilde/mdfm
```

## :inbox_tray: GitHub Releases & Update

Download prebuilt archives (`.tar.gz`, `.zip`), SHA-256 checksum files (`.sha256`), and `.deb` packages for Linux, macOS, and Windows from the [GitHub Releases](https://github.com/nicholaswilde/mdfm/releases) page.

Each release archive includes the `mdfm` binary, `README.md`, `LICENSE`, `config.example.toml`, man page (`assets/mdfm.1`), and pre-generated shell completions under `completions/`.

Update `mdfm` in-place directly from `nicholaswilde/mdfm` GitHub releases (useful when installed via GitHub release archives rather than a package manager):

```bash
# Check if a newer release is available without installing:
mdfm update --check

# Update interactively to the latest release:
mdfm update

# Update without confirmation prompt:
mdfm update --yes

# Install a specific release version or tag:
mdfm update --version 0.1.0 --yes
```

## :crab: Build or Install from Source

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (Edition 2021, stable)
- [Task](https://taskfile.dev/) (optional, recommended)

### Install or Build Locally

```bash title="Install with Cargo"
cargo install --git https://github.com/nicholaswilde/mdfm
```

```bash title="Build locally"
git clone https://github.com/nicholaswilde/mdfm.git
cd mdfm
task build:local
# Or directly with Cargo:
cargo build --release
```

The compiled binary will be placed at `target/release/mdfm`.

## :books: Shell Completions

Generate shell completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish using `mdfm completions`:

=== "Bash"

    ```bash
    # Add to ~/.bashrc for interactive sessions:
    eval "$(mdfm completions bash)"

    # Or install to standard completion directory:
    mdfm completions bash > ~/.local/share/bash-completion/completions/mdfm
    ```

=== "Zsh"

    ```bash
    # Add to ~/.zshrc (after compinit):
    source <(mdfm completions zsh)

    # Or save to your fpath directory:
    mdfm completions zsh > ~/.zfunc/_mdfm
    # (Ensure fpath includes ~/.zfunc and autoload -Uz compinit && compinit is called)
    ```

=== "Fish"

    ```fish
    # Save to standard fish completion directory:
    mdfm completions fish > ~/.config/fish/completions/mdfm.fish

    # Or load directly in current session:
    mdfm completions fish | source
    ```

=== "PowerShell"

    ```powershell
    # Append to your PowerShell profile:
    mdfm completions powershell >> $PROFILE

    # Or load into active session:
    mdfm completions powershell | Out-String | Invoke-Expression
    ```

=== "Elvish"

    ```elvish
    # Append to your Elvish rc:
    mdfm completions elvish >> ~/.elvish/rc.elv

    # Or load into active session:
    eval (mdfm completions elvish | slurp)
    ```

Pre-generated completion scripts are also checked into the [`completions/`](https://github.com/nicholaswilde/mdfm/tree/main/completions) folder.

## :page_facing_up: Man Page

Generate or view the UNIX man page:

```bash
# Generate man page
task man
# Or with binary
mdfm man > assets/mdfm.1

# Preview formatted man page
man -l assets/mdfm.1
```
