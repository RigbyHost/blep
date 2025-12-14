use base64::{engine::general_purpose, Engine as _};
use reqwest::blocking::Client;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use walkdir::WalkDir;

const MAC_GD_URL: &str = "https://cdn.rigby.host/GeometryDash.app.zip";
const WIN_GD_URL: &str = "https://cdn.rigby.host/GeometryDash.zip";
const ORIGINAL_URL: &str = "https://www.boomlings.com/database/";

pub fn patch_game(app_handle: tauri::AppHandle, id: String) -> Result<String, String> {
    let gdps_url = format!("https://gdps.rigby.host/{}/db////", id);

    // 1. Validate URL length and Pad
    let mut new_bytes = gdps_url.as_bytes().to_vec();
    if new_bytes.len() > ORIGINAL_URL.len() {
        return Err(format!(
            "URL too long! Max: {}, Got: {}",
            ORIGINAL_URL.len(),
            new_bytes.len()
        ));
    }

    // Pad with nulls if shorter
    if new_bytes.len() < ORIGINAL_URL.len() {
        new_bytes.resize(ORIGINAL_URL.len(), 0);
    }

    // 2. Locate or Download Geometry Dash
    let (base_path, _) = find_or_download_gd(&app_handle).map_err(|e| e.to_string())?;

    // 3. Prepare Server Directory
    let server_id = id.clone();
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let servers_dir = app_data_dir.join("servers");
    let target_dir = servers_dir.join(&server_id);

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    // 4. Copy Game
    // Go code logic: copy base bundle/files into target dir
    // On macOS: copy bundle to target_dir/Geometry Dash.app
    let target_binary_path = if cfg!(target_os = "macos") {
        let bundle_name = base_path.file_name().ok_or("Invalid base path")?;
        let dest_bundle = target_dir.join(bundle_name);
        copy_dir_recursive(&base_path, &dest_bundle).map_err(|e| e.to_string())?;
        dest_bundle.join("Contents/MacOS/Geometry Dash")
    } else {
        // Windows/Other: copy contents of base_path (dir) to target_dir
        // OR if base_path is binary, copy it?
        // Let's assume on Windows find_or_download_gd returns the directory containing the exe
        copy_dir_recursive(&base_path, &target_dir).map_err(|e| e.to_string())?;
        // Find exe
        target_dir.join("GeometryDash.exe")
    };

    // 5. Patch Binary
    let mut data = fs::read(&target_binary_path).map_err(|e| e.to_string())?;

    let original_bytes = ORIGINAL_URL.as_bytes();

    let original_b64 = general_purpose::STANDARD.encode(original_bytes);
    let new_b64 = general_purpose::STANDARD.encode(&new_bytes);

    // Replace URL
    let url_positions = find_all_positions(&data, original_bytes);
    // Replace all except last one (Go logic)
    if url_positions.len() > 1 {
        for &pos in url_positions.iter().take(url_positions.len() - 1) {
            data[pos..pos + original_bytes.len()].copy_from_slice(&new_bytes);
        }
    }

    // Replace Base64
    let b64_positions = find_all_positions(&data, original_b64.as_bytes());
    if b64_positions.len() > 1 {
        for &pos in b64_positions.iter().take(b64_positions.len() - 1) {
            data[pos..pos + original_b64.len()].copy_from_slice(new_b64.as_bytes());
        }
    }

    fs::write(&target_binary_path, data).map_err(|e| e.to_string())?;

    // 6. Resign (macOS)
    if cfg!(target_os = "macos") {
        // path to bundle
        let bundle_path = target_binary_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        resign_app(bundle_path).map_err(|e| e.to_string())?;
    }

    Ok(server_id)
}

pub fn run_game(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri::Manager;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let servers_dir = app_data_dir.join("servers");
    let server_dir = servers_dir.join(&id);

    if !server_dir.exists() {
        return Err("Server not installed".to_string());
    }

    if cfg!(target_os = "macos") {
        let bundle_path = server_dir.join("GeometryDash.app");
        if !bundle_path.exists() {
            return Err("Game bundle not found".to_string());
        }
        Command::new("open")
            .arg(bundle_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else {
        // Windows logic
        let exe_path = server_dir.join("GeometryDash.exe");
        if !exe_path.exists() {
            return Err("Game executable not found".to_string());
        }
        Command::new(exe_path)
            .current_dir(&server_dir) // Important for GD to find resources
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn find_all_positions(data: &[u8], pattern: &[u8]) -> Vec<usize> {
    data.windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| *window == pattern)
        .map(|(i, _)| i)
        .collect()
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(src).unwrap();
        let target_path = dst.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target_path)?;
        } else {
            fs::copy(entry.path(), &target_path)?;

            // Preserve permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(entry.path())?;
                let permissions = metadata.permissions();
                fs::set_permissions(&target_path, permissions)?;
            }
        }
    }
    Ok(())
}

fn find_or_download_gd(app_handle: &tauri::AppHandle) -> io::Result<(PathBuf, bool)> {
    // Check cache
    use tauri::Manager;
    let cache_dir = app_handle.path().app_cache_dir().unwrap();
    let gd_cache = cache_dir.join("GeometryDash");

    // Check various paths (simplified port)
    // macOS: Look for /Applications/GeometryDash.app etc.
    // Assuming we download if not found quickly.

    let expected_path = if cfg!(target_os = "macos") {
        gd_cache.join("GeometryDash.app")
    } else {
        gd_cache.join("GeometryDash")
    };

    if expected_path.exists() {
        return Ok((expected_path, cfg!(target_os = "macos")));
    }

    // Download
    fs::create_dir_all(&gd_cache)?;
    let url = if cfg!(target_os = "macos") {
        MAC_GD_URL
    } else {
        WIN_GD_URL
    };

    let resp = Client::new()
        .get(url)
        .send()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let bytes = resp
        .bytes()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Save zip
    let zip_path = gd_cache.join("temp_gd.zip");
    fs::write(&zip_path, bytes)?;

    // unzip
    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = gd_cache.join(file.name());

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        // set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    fs::remove_file(zip_path)?;

    // Fix executable permissions on macOS
    #[cfg(target_os = "macos")]
    {
        let binary_path = expected_path.join("Contents/MacOS/Geometry Dash");
        if binary_path.exists() {
            Command::new("chmod")
                .args(&["+x"])
                .arg(&binary_path)
                .output()?;
        }
    }

    Ok((expected_path, cfg!(target_os = "macos")))
}

fn resign_app(path: &Path) -> io::Result<()> {
    // Remove quarantine
    Command::new("xattr")
        .args(&["-rd", "com.apple.quarantine"])
        .arg(path)
        .output()?;
    // Remove sig
    Command::new("codesign")
        .args(&["--remove-signature"])
        .arg(path)
        .output()?;
    // Resign
    Command::new("codesign")
        .args(&["--force", "--deep", "--sign", "-"])
        .arg(path)
        .output()?;
    Ok(())
}
