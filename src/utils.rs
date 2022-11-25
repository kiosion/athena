
use std::{fmt::Write, time::Duration};
use indicatif::{ProgressBar, ProgressStyle, HumanDuration, ProgressState};

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
        .template("{spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] ({percent}%) ({smoothed_eta} remaining)")
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
