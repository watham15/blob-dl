pub mod yt_playlist;
pub mod yt_video;

use dialoguer::console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;

/// Asks the user whether they want to download video files or audio-only
fn get_media_selection(term: &Term) -> Result<MediaSelection, std::io::Error> {
    let download_formats = &[
        "Normal Video",
        "Audio-only",
        "Video-only"
    ];

    // Ask the user which format they want the downloaded files to be in
    let media_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to download video file(s) or audio-only?")
        .default(0)
        .items(download_formats)
        .interact_on(term)?;

    match media_selection {
        0 => Ok(MediaSelection::Video),
        1 => Ok(MediaSelection::AudioOnly),
        2 => Ok(MediaSelection::VideoOnly),
        _ => panic!("Error getting media selection")
    }
}

use spinoff;
use std::process;
// Running yt-dlp -j <...>
use execute::Execute;

/// Returns the output of <yt-dlp -j url>: a JSON dump of all the available format information for a video
fn get_ytdlp_formats(url: &str) -> Result<process::Output, std::io::Error> {
    // Neat animation to entertain the user while the information is being downloaded
    let sp = spinoff::Spinner::new(spinoff::Spinners::Dots10, "Fetching available formats...", spinoff::Color::Cyan);

    let mut command = process::Command::new("yt-dlp");
    // Get a JSON dump of all the available formats related to this url
    command.arg("-j");
    // Continue even if you get errors
    command.arg("-i");
    command.arg(url);
    // Redirect the output to a variable instead of the screen
    command.stdout(process::Stdio::piped());
    let output = command.execute_output();
    sp.stop();

    output
}


/// Serializes the information about the formats available for 1 video
fn serialize_formats(json_dump: &str) -> serde_json::Result<VideoSpecs> {
    // todo videos which require 18 years to see make ugly errors pop up
    // todo test if this works
    serde_json::from_str(json_dump)
}


/// Whether the user wants to download video files or audio-only
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum MediaSelection {
    Video,
    VideoOnly,
    AudioOnly,
}

/// All the information about a particular video format
#[derive(Deserialize, Serialize, Debug, PartialOrd, PartialEq)]
struct VideoFormat {
    format_id: String,
    // File extension
    ext: String,
    // Fps count, is null for audio-only formats
    fps: Option<f64>,
    // How many audio channels are available, is null for video-only formats. Unavailable on weird sb* formats
    audio_channels: Option<u64>,
    // Video resolution, is "audio only" for audio-only formats
    resolution: String,
    // Measured in MB. Unavailable on sb* formats
    filesize: Option<u64>,
    // Video codec, can be "none"
    vcodec: String,
    // Audio codec, can be "none"
    acodec: String,
    // Things like 144p, ultralow, low
    format_note: String,
    // Codec container
    container: Option<String>,
    // Total average bitrate
    tbr: Option<f64>,
    // When filesize is null, this may be available
    filesize_approx: Option<u64>,
}

impl fmt::Display for VideoFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result;

        if let Some(tbr) = self.tbr {
            // Skip picture formats
            // Add container
            result = format!("{:<6} ", self.ext);

            if self.resolution != "audio only" {
                result = format!("{}| {:<13} ", result, self.resolution);
            }

            // This isn't a picture format so unwrap() is safe
            let filesize = match self.filesize {
                Some(f) => f,
                None => self.filesize_approx.expect("Problem with filesize fetching"),
            };
            // filesize is converted from bytes to MB
            let filesize_section = format!("| filesize: {:<.2}MB", filesize as f32 * 0.000001);
            result = format!("{}{:<24}", result, filesize_section);

            // If available, add audio channels
            if let Some(ch) = self.audio_channels {
                result = format!("{}| {} audio ch ", result, ch);
            }

            result = format!("{}| tbr: {:<8.2} ", result, tbr);

            if self.vcodec != "none" {
                result = format!("{}| vcodec: {:<13} ", result, self.vcodec);
            }
            if self.acodec != "none" {
                result = format!("{}| acodec: {:<13} ", result, self.acodec);
            }

            #[cfg(debug_assertions)]
            return {
                result = format!("[[DEBUG code: {:<3}]] {} ", self.format_id, result);
                write!(f, "{}", result)
            };

            #[cfg(not(debug_assertions))]
            write!(f, "{}", result)
        } else {
            write!(f, "I shouldn't show up because I am a picture format")
        }
    }
}

// A list of all the formats available for a single video
#[derive(Deserialize, Serialize, Debug)]
struct VideoSpecs {
    formats: Vec<VideoFormat>,
}
impl VideoSpecs {
    fn formats(&self) -> &Vec<VideoFormat> {
        &self.formats
    }
}

#[derive(Debug)]
/// What quality and format the user wants a specific video to be downloaded in
pub(crate) enum VideoQualityAndFormatPreferences {
    // Code of the selected format
    UniqueFormat(String),
    // Recode the downloaded file to this format (post-processor)
    ConvertTo(String),
    BestQuality,
    SmallestSize,
}
