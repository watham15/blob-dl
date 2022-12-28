use crate::assembling;

/// Contains all the information needed to download a youtube video [WIP]
#[derive(Debug)]
pub(crate) struct ConfigYtVideo {
    url: String,
    download_format: String,
    output_path: String,
}

impl ConfigYtVideo {
    pub(crate) fn new(url: String, download_format: String, output_path: String) -> ConfigYtVideo {
        ConfigYtVideo { url, download_format, output_path }
    }
    /// Builds a yt-dl command with the needed specifications
    pub(crate) fn build_command(&self) -> std::process::Command {
        todo!()
    }
}
