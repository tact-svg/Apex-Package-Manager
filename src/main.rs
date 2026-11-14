use bzip2::read::BzDecoder;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;
use std::process::Command;
use tar::Archive;

#[derive(Parser)]
#[command(name = "apex")]
#[command(about = "Apex: Blazing-fast atomic universal package manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search available packages
    Search { query: String },
    /// Install any package locally or via universal Nix registry
    Install { name: String },
    /// Remove an installed package via universal bridge
    Remove { name: String },
    /// Refresh registries and update packages
    Update,
    /// Upgrade all universal packages via Apex bridge
    Upgrade,
}

struct MiniMoss {
    root: PathBuf,
    store_dir: PathBuf,
    states_dir: PathBuf,
    active_link: PathBuf,
}

impl MiniMoss {
    fn new(root: PathBuf) -> Self {
        let store_dir = root.join("store");
        let states_dir = root.join("states");
        let active_link = root.join("active");

        let _ = fs::create_dir_all(&store_dir);
        let _ = fs::create_dir_all(&states_dir);

        Self { root, store_dir, states_dir, active_link }
    }

    fn ingest_bytes(&self, content: &[u8], executable: bool) -> String {
        let hash = blake3::hash(content).to_hex().to_string();
        let blob_path = self.store_dir.join(&hash);

        if !blob_path.exists() {
            fs::write(&blob_path, content).expect("Failed to write to store");
            if executable {
                let mut perms = fs::metadata(&blob_path).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&blob_path, perms).unwrap();
            }
        }
        hash
    }

    fn create_state(&self, state_id: u32, files: &[(&str, &str)]) {
        let state_path = self.states_dir.join(state_id.to_string());
        if state_path.exists() {
            let _ = fs::remove_dir_all(&state_path);
        }
        fs::create_dir_all(&state_path).unwrap();

        for (rel_path, hash) in files {
            let blob_path = self.store_dir.join(hash);
            let target_path = state_path.join(rel_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let _ = fs::hard_link(&blob_path, &target_path);
        }
    }

    fn activate_state(&self, state_id: u32) {
        let target_state = self.states_dir.join(state_id.to_string());
        let temp_link = self.root.join("active_tmp");

        if temp_link.exists() {
            let _ = fs::remove_file(&temp_link);
        }
        let _ = symlink(&target_state, &temp_link);
        let _ = fs::rename(&temp_link, &self.active_link);
    }
}

struct Package {
    name: &'static str,
    version: &'static str,
    url: &'static str,
    binary_path: &'static str,
}

fn get_catalog() -> Vec<Package> {
    vec![
        Package { name: "btop", version: "1.4.0", url: "https://github.com/aristocratos/btop/releases/download/v1.4.0/btop-x86_64-linux-musl.tbz", binary_path: "btop/bin/btop" },
        Package { name: "fastfetch", version: "2.30.0", url: "https://github.com/fastfetch-cli/fastfetch/releases/download/2.30.0/fastfetch-linux-amd64.tar.gz", binary_path: "fastfetch-linux-amd64/usr/bin/fastfetch" },
        Package { name: "ripgrep", version: "14.1.0", url: "https://github.com/BurntSushi/ripgrep/releases/download/14.1.0/ripgrep-14.1.0-x86_64-unknown-linux-musl.tar.gz", binary_path: "ripgrep-14.1.0-x86_64-unknown-linux-musl/rg" },
        Package { name: "bat", version: "0.24.0", url: "https://github.com/sharkdp/bat/releases/download/v0.24.0/bat-v0.24.0-x86_64-unknown-linux-musl.tar.gz", binary_path: "bat-v0.24.0-x86_64-unknown-linux-musl/bat" },
        Package { name: "lazygit", version: "0.40.2", url: "https://github.com/jesseduffield/lazygit/releases/download/v0.40.2/lazygit_0.40.2_Linux_x86_64.tar.gz", binary_path: "lazygit" },
    ]
}

fn ensure_unfree_config() -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    let mut config_dir = PathBuf::from(home);
    config_dir.push(".config");
    config_dir.push("nixpkgs");
    
    fs::create_dir_all(&config_dir)?;
    let config_file = config_dir.join("config.nix");
    
    if !config_file.exists() {
        fs::write(&config_file, "{ allowUnfree = true; }\n")?;
        println!("✔ Global Nix unfree configuration created.");
    }
    Ok(())
}

fn remove_package(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️ Searching profile for '{}'...", package_name);

    let list_output = Command::new("sh")
        .arg("-c")
        .arg(". /etc/profile.d/nix.sh 2>/dev/null || . $HOME/.nix-profile/etc/profile.d/nix.sh 2>/dev/null; nix profile list")
        .output()?;

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    let mut target_index = None;

    for line in stdout.lines() {
        if line.contains(package_name) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(idx_str) = parts.first() {
                let clean_idx = idx_str.trim_end_matches(':');
                if clean_idx.parse::<usize>().is_ok() {
                    target_index = Some(clean_idx.to_string());
                    break;
                }
            }
        }
    }

    if let Some(idx) = target_index {
        println!("🗑️ Removing package index '{}' matching '{}'...", idx, package_name);
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!(
                ". /etc/profile.d/nix.sh 2>/dev/null || . $HOME/.nix-profile/etc/profile.d/nix.sh 2>/dev/null; nix profile remove {}",
                idx
            ))
            .status()?;

        if status.success() {
            println!("✅ Successfully removed '{}' via Apex!", package_name);
        } else {
            println!("❌ Failed to remove package from Nix profile.");
        }
    } else {
        println!("❌ Package '{}' not found in your Nix profile.", package_name);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let home = env::var("HOME").expect("HOME not set");
    let apex_root = PathBuf::from(&home).join(".apex");
    let moss = MiniMoss::new(apex_root.clone());
    let catalog = get_catalog();

    match cli.command {
        Commands::Search { query } => {
            println!("🔍 Searching local curated catalog & universal index...");
            let matches: Vec<&Package> = catalog.iter()
                .filter(|p| p.name.contains(&query.to_lowercase()))
                .collect();

            for pkg in matches {
                println!("  📦 [Local Fast-Path] {} (v{})", pkg.name, pkg.version);
            }
            println!("  🌐 [Universal Registry] Any other package is available via nixpkgs (80,000+ packages). Try: apex install <pkg>");
        }
        Commands::Install { name } => {
            let local_pkg = catalog.into_iter().find(|p| p.name == name.to_lowercase());

            if let Some(pkg) = local_pkg {
                println!("⚡ [Local Fast-Path] Downloading '{}' (v{})...", pkg.name, pkg.version);
                let resp = ureq::get(pkg.url).call().expect("Download failed");
                let mut archive_bytes = Vec::new();
                resp.into_reader().read_to_end(&mut archive_bytes).unwrap();

                let mut binary_bytes = Vec::new();
                if pkg.url.ends_with(".tbz") || pkg.url.ends_with(".bz2") {
                    let tar = BzDecoder::new(&archive_bytes[..]);
                    let mut archive = Archive::new(tar);
                    for entry in archive.entries().unwrap() {
                        let mut entry = entry.unwrap();
                        if entry.path().unwrap().ends_with(pkg.binary_path) {
                            entry.read_to_end(&mut binary_bytes).unwrap();
                            break;
                        }
                    }
                } else {
                    let dec = flate2::read::GzDecoder::new(&archive_bytes[..]);
                    let mut archive = Archive::new(dec);
                    for entry in archive.entries().unwrap() {
                        let mut entry = entry.unwrap();
                        if entry.path().unwrap().ends_with(pkg.binary_path) {
                            entry.read_to_end(&mut binary_bytes).unwrap();
                            break;
                        }
                    }
                }

                let hash = moss.ingest_bytes(&binary_bytes, true);
                let bin_rel_path = format!("bin/{}", pkg.name);
                moss.create_state(1, &[(&bin_rel_path, &hash)]);
                moss.activate_state(1);

                println!("🚀 APEX: Successfully installed '{}' locally via MiniMoss!", pkg.name);
            } else {
                let _ = ensure_unfree_config();

                let check_nix = Command::new("nix").arg("--version").status();
                if check_nix.is_err() || !check_nix.unwrap().success() {
                    println!("⚡ Nix backend not found. Downloading universal installer script...");
                    let _ = Command::new("curl")
                        .arg("-sSfL")
                        .arg("https://install.determinate.systems/nix")
                        .arg("-o")
                        .arg("/tmp/nix-install.sh")
                        .status();

                    println!("⚡ Running universal installer (requires sudo password)...");
                    let bootstrap = Command::new("sh")
                        .arg("/tmp/nix-install.sh")
                        .arg("install")
                        .status();

                    if bootstrap.is_err() || !bootstrap.unwrap().success() {
                        println!("❌ Failed to bootstrap Nix backend.");
                        return;
                    }
                }

                println!("🌐 Installing '{}' via Universal Registry bridge...", name);
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        ". /etc/profile.d/nix.sh 2>/dev/null || . $HOME/.nix-profile/etc/profile.d/nix.sh 2>/dev/null; NIXPKGS_ALLOW_UNFREE=1 nix profile install --impure nixpkgs#{}",
                        name
                    ))
                    .status();

                match status {
                    Ok(s) if s.success() => println!("✅ Successfully installed '{}' via Apex Universal Bridge!", name),
                    _ => println!("❌ Package '{}' not found in registry.", name),
                }
            }
        }
        Commands::Remove { name } => {
            if let Err(e) = remove_package(&name) {
                eprintln!("❌ Error: {}", e);
            }
        }
        Commands::Update => {
            let _ = ensure_unfree_config();
            println!("🔄 Refreshing package registries and fetching latest versions...");
            let status = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    ". /etc/profile.d/nix.sh 2>/dev/null || . $HOME/.nix-profile/etc/profile.d/nix.sh 2>/dev/null; NIXPKGS_ALLOW_UNFREE=1 nix profile upgrade --all --refresh --impure"
                ))
                .status();

            match status {
                Ok(s) if s.success() => println!("✅ Registries refreshed and packages updated successfully!"),
                _ => println!("❌ Failed to refresh packages."),
            }
        }
        Commands::Upgrade => {
            let _ = ensure_unfree_config();
            println!("⬆️ Upgrading all universal packages via Apex bridge...");
            let status = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    ". /etc/profile.d/nix.sh 2>/dev/null || . $HOME/.nix-profile/etc/profile.d/nix.sh 2>/dev/null; NIXPKGS_ALLOW_UNFREE=1 nix profile upgrade --all --impure"
                ))
                .status();

            match status {
                Ok(s) if s.success() => println!("✅ All packages upgraded successfully!"),
                _ => println!("❌ Failed to upgrade packages."),
            }
        }
    }
}
