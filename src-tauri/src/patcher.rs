use base64::{engine::general_purpose, Engine as _};
use reqwest::blocking::Client;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::Manager;
use walkdir::WalkDir;

const MAC_GD_URL: &str = "https://cdn.rigby.host/GeometryDash.app.zip";
const WIN_GD_URL: &str = "https://cdn.rigby.host/GeometryDash.zip";
const ORIGINAL_URL: &str = "https://www.boomlings.com/database/";

pub fn patch_game(app_handle: tauri::AppHandle, id: String) -> Result<String, String> {
    let gdps_url = format!("https://gdps.rigby.host/{}/db////", id);

    let mut new_bytes = gdps_url.as_bytes().to_vec();
    if new_bytes.len() > ORIGINAL_URL.len() {
        return Err(format!(
            "URL too long! Max: {}, Got: {}",
            ORIGINAL_URL.len(),
            new_bytes.len()
        ));
    }

    if new_bytes.len() < ORIGINAL_URL.len() {
        new_bytes.resize(ORIGINAL_URL.len(), 0);
    }

    let (base_path, _) = find_or_download_gd(&app_handle).map_err(|e| e.to_string())?;

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

    let target_binary_path = if cfg!(target_os = "macos") {
        let bundle_name = base_path.file_name().ok_or("Invalid base path")?;
        let dest_bundle = target_dir.join(bundle_name);
        copy_dir_recursive(&base_path, &dest_bundle).map_err(|e| e.to_string())?;
        dest_bundle.join("Contents/MacOS/Geometry Dash")
    } else {
        copy_dir_recursive(&base_path, &target_dir).map_err(|e| e.to_string())?;
        target_dir.join("GeometryDash.exe")
    };

    let mut data = fs::read(&target_binary_path).map_err(|e| e.to_string())?;
    let original_bytes = ORIGINAL_URL.as_bytes();
    let original_b64 = general_purpose::STANDARD.encode(original_bytes);
    let new_b64 = general_purpose::STANDARD.encode(&new_bytes);

    let url_positions = find_all_positions(&data, original_bytes);
    if url_positions.len() > 1 {
        for &pos in url_positions.iter().take(url_positions.len() - 1) {
            data[pos..pos + original_bytes.len()].copy_from_slice(&new_bytes);
        }
    }

    let b64_positions = find_all_positions(&data, original_b64.as_bytes());
    if b64_positions.len() > 1 {
        for &pos in b64_positions.iter().take(b64_positions.len() - 1) {
            data[pos..pos + original_b64.len()].copy_from_slice(new_b64.as_bytes());
        }
    }

    fs::write(&target_binary_path, data).map_err(|e| e.to_string())?;

    if cfg!(target_os = "macos") {
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
        let exe_path = server_dir.join("GeometryDash.exe");
        if !exe_path.exists() {
            return Err("Game executable not found".to_string());
        }
        Command::new(exe_path)
            .current_dir(&server_dir)
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

fn find_windows_gd_root(gd_cache: &Path) -> io::Result<PathBuf> {
    if !gd_cache.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "GeometryDash cache directory not found",
        ));
    }

    let direct_exe = gd_cache.join("GeometryDash.exe");
    if direct_exe.is_file() {
        return Ok(gd_cache.to_path_buf());
    }

    let nested_dir = gd_cache.join("GeometryDash");
    if nested_dir.join("GeometryDash.exe").is_file() {
        return Ok(nested_dir);
    }

    for entry in WalkDir::new(gd_cache).min_depth(1) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.eq_ignore_ascii_case("GeometryDash.exe") {
                    if let Some(parent) = entry.path().parent() {
                        return Ok(parent.to_path_buf());
                    }
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "GeometryDash.exe not found in cache",
    ))
}

fn find_or_download_gd(app_handle: &tauri::AppHandle) -> io::Result<(PathBuf, bool)> {
    let cache_dir = app_handle.path().app_cache_dir().unwrap();
    let gd_cache = cache_dir.join("GeometryDash");

    if cfg!(target_os = "macos") {
        let expected_path = gd_cache.join("GeometryDash.app");
        if expected_path.exists() {
            return Ok((expected_path, true));
        }
    } else if let Ok(root_path) = find_windows_gd_root(&gd_cache) {
        return Ok((root_path, false));
    }

    fs::create_dir_all(&gd_cache)?;
    let url = if cfg!(target_os = "macos") {
        MAC_GD_URL
    } else {
        WIN_GD_URL
    };

    let client = Client::builder().timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let bytes = resp
        .bytes()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let zip_path = gd_cache.join("temp_gd.zip");
    fs::write(&zip_path, bytes)?;

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

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    fs::remove_file(zip_path)?;

    #[cfg(target_os = "macos")]
    {
        let binary_path = gd_cache
            .join("GeometryDash.app")
            .join("Contents/MacOS/Geometry Dash");
        if binary_path.exists() {
            Command::new("chmod")
                .args(&["+x"])
                .arg(&binary_path)
                .output()?;
        }
    }

    if cfg!(target_os = "macos") {
        Ok((gd_cache.join("GeometryDash.app"), true))
    } else {
        let root_path = find_windows_gd_root(&gd_cache)?;
        Ok((root_path, false))
    }
}

fn resign_app(path: &Path) -> io::Result<()> {
    Command::new("xattr")
        .args(&["-rd", "com.apple.quarantine"])
        .arg(path)
        .output()?;
    Command::new("codesign")
        .args(&["--remove-signature"])
        .arg(path)
        .output()?;
    Command::new("codesign")
        .args(&["--force", "--deep", "--sign", "-"])
        .arg(path)
        .output()?;
    Ok(())
}
