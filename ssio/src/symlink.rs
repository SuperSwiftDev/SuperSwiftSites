use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

#[cfg(windows)]
use std::os::windows::fs as windows_fs;

use pathdiff::diff_paths;

const DEBUG_MODE: bool = false;

/// Create a symbolic link at `link_path` pointing to `target_path`.
/// Handles platform differences and ensures parent directories exist.
pub fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(target_path: P, link_path: Q) -> std::io::Result<()> {
    let target_path = target_path.as_ref();
    let link_path = link_path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Remove existing file or link
    if link_path.exists() {
        fs::remove_file(link_path)?;
    }

    #[cfg(unix)]
    {
        symlink(target_path, link_path)?;
    }

    #[cfg(windows)]
    {
        symlink_file(target_path, link_path)?;
    }

    Ok(())
}

// /// Create a relative symlink from `link_path` to `real_path`.
// pub fn create_relative_symlink(real_path: &Path, link_path: &Path) -> std::io::Result<()> {
//     // Ensure parent directory exists
//     if let Some(parent) = link_path.parent() {
//         fs::create_dir_all(parent)?;
//     }

//     // Clean up any existing link
//     if link_path.exists() {
//         fs::remove_file(link_path)?;
//     }

//     // Compute relative path from link's parent to real path
//     let link_dir = link_path.parent().expect("Link has no parent");
//     let relative_target = pathdiff::diff_paths(real_path, link_dir)
//         .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to compute relative path"))?;

//     #[cfg(unix)]
//     {
//         symlink(&relative_target, &link_path)?;
//     }

//     #[cfg(windows)]
//     {
//         symlink_file(&relative_target, &link_path)?;
//     }

//     println!(
//         "Symlink created: {:?} → {:?}",
//         link_path.display(),
//         relative_target.display()
//     );

//     Ok(())
// }



/// Create a relative symlink from `link_path` to `real_path`, only if needed.
pub fn create_relative_symlink(real_path: impl AsRef<Path>, link_path: impl AsRef<Path>) -> std::io::Result<()> {
    let real_path= real_path.as_ref();
    let link_path = link_path.as_ref();
    // let real_path= path_clean::clean(real_path);
    // let link_path = path_clean::clean(link_path);
    let link_dir = link_path.parent().expect("Link must have a parent");

    // Ensure link directory exists
    fs::create_dir_all(link_dir).unwrap();

    // Compute relative path from symlink location to real target
    let relative_target = diff_paths(real_path, link_dir)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to compute relative path"))
        .unwrap();

    // Check if symlink exists and is correct
    if link_path.exists() {
        // Read existing symlink target (only works if it's actually a symlink)
        #[cfg(unix)]
        let current_target = fs::read_link(&link_path)
            .unwrap_or_else(|errror| {
                panic!("FAILED TO READ SYM-LINK {link_path:?}: {errror}\n{errror:?}");
            });

        #[cfg(windows)]
        let current_target = {
            // Only treat as symlink if it's a symlink file
            use std::os::windows::fs::MetadataExt;
            let metadata = fs::symlink_metadata(link_path)?;
            if metadata.file_type().is_symlink() {
                fs::read_link(link_path)?
            } else {
                // It's a regular file — not a symlink, needs replacement
                PathBuf::new()
            }
        };

        if current_target == relative_target {
            if DEBUG_MODE {
                println!("✅ Symlink already exists and is correct: {:?}", link_path.display());
            }
            return Ok(());
        } else {
            if DEBUG_MODE {
                println!("⚠️ Symlink exists but points elsewhere. Replacing it.");
            }
            fs::remove_file(&link_path).unwrap();
        }
    }

    // Create symlink
    #[cfg(unix)]
    unix_fs::symlink(&relative_target, &link_path).unwrap();

    #[cfg(windows)]
    windows_fs::symlink_file(&relative_target, link_path)?;

    if DEBUG_MODE {
        println!(
            "🔗 Symlink created: {} → {}",
            link_path.display(),
            relative_target.display()
        );
    }

    Ok(())
}
