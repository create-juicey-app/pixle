use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Deserialize, Clone)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedTool {
    pub name: String,
    pub script_content: String,
    pub package_path: PathBuf,
}

pub struct PackageManager {
    pub tools: Vec<LoadedTool>,
}

impl PackageManager {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn load_packages(&mut self) {
        let packages_dir = Path::new("packages");

        // Just fail silently/print if no folder exists
        if !packages_dir.exists() {
            println!("No 'packages' folder found.");
            return;
        }

        println!("--- Scanning Packages ---");
        for entry in fs::read_dir(packages_dir).expect("Failed to read packages dir") {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                self.load_single_package(&path);
            }
        }
    }

    fn load_single_package(&mut self, path: &Path) {
        let manifest_path = path.join("manifest.toml");
        if !manifest_path.exists() {
            return;
        }

        let manifest_str = fs::read_to_string(&manifest_path).unwrap_or_default();
        if let Ok(manifest) = toml::from_str::<PackageManifest>(&manifest_str) {
            println!("Found Package: {} v{}", manifest.name, manifest.version);
        }

        let tools_path = path.join("tools");
        if tools_path.exists() {
            for entry in WalkDir::new(tools_path) {
                let entry = entry.unwrap();
                let f_path = entry.path();

                if f_path.extension().and_then(|s| s.to_str()) == Some("lua") {
                    let script = fs::read_to_string(f_path).unwrap();
                    let tool_name = f_path.file_stem().unwrap().to_str().unwrap().to_string();

                    self.tools.push(LoadedTool {
                        name: tool_name,
                        script_content: script,
                        package_path: path.to_path_buf(),
                    });
                }
            }
        }
    }
}
