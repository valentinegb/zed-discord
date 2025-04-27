extern crate zed_extension_api as zed;

use std::{
    fs::{metadata, read_dir, remove_dir_all, remove_file},
    path::PathBuf,
};

use zed::{
    Architecture, DownloadedFileType, Extension, GithubReleaseOptions, LanguageServerId,
    LanguageServerInstallationStatus, Os, Worktree, current_platform, download_file,
    latest_github_release, make_file_executable, register_extension,
    set_language_server_installation_status,
};

struct DiscordExtension {
    cached_language_server_path: Option<PathBuf>,
}

impl DiscordExtension {
    fn language_server_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<PathBuf> {
        if let Some(cached_language_server_path) = &self.cached_language_server_path {
            return Ok(cached_language_server_path.clone());
        }

        if let Some(language_server_path) = worktree.which("discord-ls") {
            let language_server_path = PathBuf::from(language_server_path);

            self.cached_language_server_path = Some(language_server_path.clone());

            return Ok(language_server_path);
        }

        set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let latest_release = latest_github_release(
            "valentinegb/discord-ls",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;
        let (os, arch) = current_platform();
        let language_server_path = PathBuf::from(format!(
            "{}/discord-ls{}",
            latest_release.version,
            if os == Os::Windows { ".exe" } else { "" },
        ));

        if !metadata(&language_server_path).is_ok_and(|metadata| metadata.is_file()) {
            set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Downloading,
            );

            let os = match os {
                Os::Mac => "macOS",
                Os::Linux => "Linux",
                Os::Windows => "Windows",
            };
            let arch = match arch {
                Architecture::Aarch64 => "ARM64",
                Architecture::X8664 => "X64",
                other => return Err(format!("unsupported architecture: {other:?}")),
            };
            let download_url = latest_release
                .assets
                .into_iter()
                .find(|asset| asset.name == format!("discord-ls-{os}-{arch}.zip"))
                .ok_or("no available release asset for current platform")?
                .download_url;

            download_file(
                &download_url,
                &latest_release.version,
                DownloadedFileType::Zip,
            )?;
            make_file_executable(language_server_path.to_str().ok_or(format!(
                "could not convert language server path to string: {language_server_path:?}"
            ))?)?;

            if let Ok(read_dir) = read_dir("./") {
                for entry in read_dir.flatten() {
                    if entry.file_name() != *latest_release.version {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                remove_dir_all(entry.path()).ok();
                            } else {
                                remove_file(entry.path()).ok();
                            }
                        }
                    }
                }
            }

            set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::None,
            );
        }

        self.cached_language_server_path = Some(language_server_path.clone());

        Ok(language_server_path)
    }
}

impl Extension for DiscordExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_language_server_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command::new(
            self.language_server_path(language_server_id, worktree)?
                .into_os_string()
                .into_string()
                .map_err(|os_string| {
                    format!("could not convert language server path to string: {os_string:?}")
                })?,
        ))
    }
}

register_extension!(DiscordExtension);
