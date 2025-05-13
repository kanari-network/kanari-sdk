use std::process::Command;
use std::path::Path;
use std::env;

fn main() {
    println!("Kanari SDK Build Support Tool");
    println!("-----------------------------");

    // Check if we're in a git repository
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        println!("Error: Not in a git repository. Cannot initialize submodules.");
        return;
    }

    // Get the workspace root directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let workspace_root = current_dir.ancestors()
        .find(|path| Path::new(&path).join("Cargo.toml").exists())
        .unwrap_or(&current_dir);

    println!("Initializing submodules in workspace: {}", workspace_root.display());
    
    // Submodules to initialize
    let submodules = ["third_party/move"];
    
    for submodule in &submodules {
        let submodule_path = workspace_root.join(submodule);
        
        println!("Checking submodule: {}", submodule);
        
        if !submodule_path.exists() || submodule_path.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            println!("Initializing submodule: {}", submodule);
            
            // Initialize and update the submodule
            let status = Command::new("git")
                .current_dir(workspace_root)
                .args(&["submodule", "update", "--init", "--recursive", submodule])
                .status();
            
            match status {
                Ok(exit_status) if exit_status.success() => {
                    println!("Submodule initialization successful: {}", submodule);
                }
                Ok(_) => {
                    println!("Failed to initialize submodule: {}", submodule);
                }
                Err(e) => {
                    println!("Error running git command: {}", e);
                }
            }
        } else {
            println!("Submodule already initialized: {}", submodule);
        }
    }
    
    println!("\nDone initializing submodules!");
    println!("You can now run 'cargo build' to build the project");
}
