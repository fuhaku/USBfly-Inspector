use std::process::Command;
use std::env;
use std::path::Path;

fn main() {
    // Only run the following on macOS
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.14");
        
        // Get the output directory (for debug or release builds)
        let profile = env::var("PROFILE").unwrap();
        let out_dir = format!("target/{}", profile);
        
        // Create Resources directory if it doesn't exist
        let resources_dir = Path::new(&out_dir).join("Resources");
        if !resources_dir.exists() {
            std::fs::create_dir_all(&resources_dir).unwrap();
        }
        
        // Copy any required resources
        // For example, if you have assets that need to be included:
        // std::fs::copy("assets/some_file.json", resources_dir.join("some_file.json")).unwrap();
    }
}
