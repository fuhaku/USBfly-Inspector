// Import needed libs here when build script is expanded
use std::env;
use std::path::Path;

fn main() {
    // Run configuration based on target OS
    if cfg!(target_os = "macos") {
        // macOS specific configuration
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
    } else if cfg!(target_os = "linux") {
        // Linux specific configuration (useful for Replit)
        println!("cargo:rustc-env=WINIT_X11_SCALE_FACTOR=1.0");
        println!("cargo:rustc-env=LIBGL_ALWAYS_SOFTWARE=1");
        println!("cargo:rustc-cfg=feature=\"wayland\"");
        println!("cargo:rustc-cfg=feature=\"x11\"");
        
        // Tell iced to use the tiny skia renderer which works better in headless environments
        println!("cargo:rustc-cfg=feature=\"tiny_skia\"");
    }
}
