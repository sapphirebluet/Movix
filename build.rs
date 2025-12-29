use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ytdlp_dir = out_dir.join("ytdlp");
    fs::create_dir_all(&ytdlp_dir).unwrap();

    let (url, filename) = get_ytdlp_url();
    let ytdlp_path = ytdlp_dir.join(filename);

    if !ytdlp_path.exists() {
        download_ytdlp(url, &ytdlp_path);
    }

    println!("cargo:rustc-env=YTDLP_PATH={}", ytdlp_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}

fn get_ytdlp_url() -> (&'static str, &'static str) {
    #[cfg(target_os = "linux")]
    {
        (
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
            "yt-dlp",
        )
    }
    #[cfg(target_os = "macos")]
    {
        (
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
            "yt-dlp",
        )
    }
    #[cfg(target_os = "windows")]
    {
        (
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
            "yt-dlp.exe",
        )
    }
}

fn download_ytdlp(url: &str, path: &PathBuf) {
    println!("Downloading yt-dlp from {}", url);

    let response = reqwest::blocking::get(url).expect("Failed to download yt-dlp");
    let bytes = response.bytes().expect("Failed to read yt-dlp bytes");
    fs::write(path, &bytes).expect("Failed to write yt-dlp");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}
