use std::{fmt::Write, time::Duration};
use indicatif::{ProgressBar, ProgressStyle, HumanDuration, ProgressState};

#[derive(Clone)]
pub struct Options {
    pub verbose: bool,
    pub upload: bool,
    pub compression: bool,
    pub input_path: std::path::PathBuf,
    pub output_path: std::path::PathBuf,
}

// Generic util for prompting user for y/n input
pub fn prompt_user(message: String, prompt: String, default: Option<bool>) -> bool {
    let default = match default {
        Some(true) => "y",
        Some(false) => "n",
        None => "n",
    };
    eprintln!("{}", message);
    eprint!("{} {} ", prompt, if default == "y" { "[Y/n]" } else { "[y/N]" });
    let mut input = String::new();
    loop {
        std::io::stdin().read_line(&mut input).unwrap();
        let answer = match input.trim().to_lowercase().as_str() {
            "y" => "y",
            "yes" => "y",
            "n" => "n",
            "no" => "n",
            "" => default,
            _ => {
                eprint!("Invalid input, please enter 'y' or 'n': ");
                input.clear();
                continue;
            }
        };
        if answer == "y" || answer == "n" {
            return answer == "y";
        }
    }
    
}

pub fn construct_progress(len: u64) -> ProgressBar {
    let bar = ProgressBar::new(len);
    let style = ProgressStyle::default_bar()
        .with_key(
            "smoothed_eta",
            |s: &ProgressState, w: &mut dyn Write| match (s.pos(), s.len()) {
                (pos, Some(len)) => write!(
                    w,
                    "~{:#}",
                    HumanDuration(Duration::from_millis(
                        (s.elapsed().as_millis() * (len as u128 - pos as u128) / (std::cmp::max(1 as u128, pos as u128)))
                            as u64
                    ))
                )
                .unwrap(),
                _ => write!(w, "-").unwrap(),
            },
        )
        .with_key(
            "smoothed_per_sec",
            |s: &ProgressState, w: &mut dyn Write| match (s.pos(), s.elapsed().as_millis()) {
                (pos, elapsed_ms) if elapsed_ms > 0 => {
                    write!(w, "{:.2}/s", pos as f64 * 1000_f64 / elapsed_ms as f64).unwrap()
                }
                _ => write!(w, "-").unwrap(),
            },
        )
        .template("{spinner:.green} [{elapsed_precise}] {msg} [{wide_bar:.cyan/blue}] ({percent}%) ({smoothed_eta} remaining)")
        .unwrap()
        .tick_strings(&[".  ",".. ","..."," ..","  .","   "])
        .progress_chars("=>-");
    bar.set_style(style);
    bar
}

pub fn construct_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[".  ",".. ","..."," ..","  .","   "])
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner
}
