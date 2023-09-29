use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use dialoguer::console::Term;
use std::collections::HashMap;
use colored::Colorize;

use crate::youtube_error_message::*;
use crate::ui_prompts::*;
use crate::parser;
use crate::error::YtdlpError;
use crate::assembling::youtube::config;

/// Executes the yt-dlp command and analyzes its output.
///
/// It filters what to show to the user according to verbosity options
///
/// It records which videos fail to download and the reason: if trying again can fix the issue the user can choose to retry
pub fn run_and_observe(command: &mut Command, download_config: &config::DownloadConfig, verbosity: &parser::Verbosity) {
    // Run the command and record any errors
    if let Some(errors) = run_command(command, verbosity) {
        // Some videos could not be downloaded, ask the user which ones they want to try to re-download
        let user_selection = ask_for_redownload(&errors);

        // The list of commands that have to be re-run in case of errors
        let mut to_be_downloaded = Vec::new();

        // Selection 0 and 1 are hard-coded (select all | select nothing)
        if !user_selection.is_empty() {
            if user_selection[0] == 0 {
                // The user wants to re-download all the videos
                for video_to_re_download in &errors {
                    // Re-download every video while keeping the current command configuration (quality, naming preference, ...)
                    to_be_downloaded.push(download_config.build_command_for_video(video_to_re_download.video_id()));
                }
            } else if user_selection[0] == 1 {
                // The user doesn't want to re-download anything
            } else {
                // Only re-download the selected videos
                for i in user_selection {
                    // Skip 0 and 1 because they are hard-coded options (select all or nothing)
                    if i == 0 || i == 1 {
                        continue;
                    }
                    // There is a 1:1 correspondence between the number in user_selection and
                    // the index of the video it refers to in errors
                    to_be_downloaded.push(download_config.build_command_for_video(errors[i - 2].video_id().as_str()));
                }
            }
        }
        for mut com in to_be_downloaded {
            run_command(&mut com, verbosity);
        }
    } else {
        #[cfg(debug_assertions)]
        println!("The command ran without any errors!! :)");
    }
}

/// Returns whether it makes sense to try downloading the video again
fn is_recoverable(error: &YtdlpError, table: &HashMap<&'static str, bool>) -> bool {
    if error.error_msg().contains(VIDEO_UNAVAILABLE) {
        return false;
    }
    if let Some(result) = table.get(error.error_msg().as_str()) {
        if *result == false {
            // The error is documented and unrecoverable
            false
        } else {
            // The error is documented and recoverable
            true
        }
    } else {
        // By default undocumented errors are flagged as recoverable
        true
    }
}

/// A list of all the documented youtube error messages and whether they are recoverable.
fn init_error_msg_lut() -> HashMap<&'static str, bool> {
    HashMap::from([
        (PRIVATE_VIDEO,          false),
        (NONEXISTENT_PLAYLIST,   false),
        (HOMEPAGE_REDIRECT,      false),
        (VIOLENT_VIDEO,          false),
        (REMOVED_VIDEO,          false),
        (YTDLP_GAVE_UP,          false),
        (VIDEO_NOT_FOUND,        false),
        (NETWORK_FAIL,           true),
        (NO_API_PAGE,            false),
        (ENCODER_STREAM_ERROR,   false),
        (NONEXISTENT_VIDEO,   false),
    ])
}

/// Runs the command and displays the output to the console.
///
/// If yt-dlp runs into any errors, they are returned in a vector of Ytdlp errors (parsed Strings)
fn run_command(command: &mut Command, verbosity: &parser::Verbosity) -> Option<Vec<YtdlpError>> {
    // Run the command and capture its output
    let mut youtube_dl = command.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start yt-dlp process");

    let stdout = BufReader::new(youtube_dl.stdout.take().unwrap());
    let stderr = BufReader::new(youtube_dl.stderr.take().unwrap());

    // All the errors produced by yt-dlp
    let mut errors: Vec<YtdlpError> = vec![];

    match verbosity {
        parser::Verbosity::Quiet   => {
            // This has to be run or the command does nothing
            for line in stdout.lines().chain(stderr.lines()) {
                let line = line.unwrap();

                // Keep track of errors without displaying anything
                if line.contains("ERROR:") {
                    errors.push(YtdlpError::from_error_output(&line));
                }
            }
        },

        parser::Verbosity::Default => {
            for line in stdout.lines().chain(stderr.lines()) {
                let line = line.unwrap();

                // Only show download/error lines
                if line.contains("[download]") {
                    println!("{}", line);
                } else if line.contains("ERROR:") {
                    errors.push(YtdlpError::from_error_output(&line));
                    // Color error messages red
                    println!("{}", line.red());
                }
            }
        },

        parser::Verbosity::Verbose => {
            // Print to the console everything that yt-dlp is doing
            for line in stdout.lines().chain(stderr.lines()) {
                let line = line.unwrap();

                if line.contains("ERROR:") {
                    errors.push(YtdlpError::from_error_output(&line));
                    // Color error messages red
                    println!("{}", line.red());
                } else {
                    println!("{}", line);
                }
            }
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

/// Shows the user which videos could not be downloaded and returns which have to be re-downloaded based on what the user wants
///
/// Returns a Vec containing which errors the user wants to re-download
fn ask_for_redownload(errors: &Vec<YtdlpError>) -> Vec<usize> {
    let term = Term::buffered_stderr();

    // Initialize a lut, which contains all documented errors and whether they can be recovered from
    let lut = init_error_msg_lut();

    // The possible choices which will be presented to the user (all recoverable errors)
    let mut user_options = Vec::new();

    let mut unrecoverable_errors = Vec::new();

    // Default options
    user_options.push(String::from(SELECT_ALL));
    user_options.push(String::from(SELECT_NOTHING));

    for error in errors {
        if is_recoverable(error, &lut) {
            // It makes sense to try a re-download
            user_options.push(error.to_string())
        } else {
            // Don't bother asking to re-download the error
            unrecoverable_errors.push(error);
        }
    }

    if !unrecoverable_errors.is_empty() {
        println!("{}", UNRECOVERABLE_ERROR_PROMPT.bold().cyan());
        for error in unrecoverable_errors {
            println!("   {}", error);
        }
    }

    if user_options.len() > 2 {
        // If user_options has only 2 elements there aren't any videos to re-download
        let user_selection = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt(ERROR_RETRY_PROMPT)
            .items(&user_options[..])
            .interact_on(&term).unwrap();

        println!("{}", DEBUG_REPORT_PROMPT);
        return user_selection
    }

    // The user didn't choose any options so an empty Vec is returned
    Vec::new()
}
