use indicatif::{ProgressBar, ProgressStyle};

pub fn bytes_bar(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("valid template")
            .progress_chars("=>-"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}

pub fn count_bar(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix} [{bar:40.green/white}] {pos}/{len} ({eta})")
            .expect("valid template")
            .progress_chars("=>-"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}

pub fn spinner(prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{prefix} {spinner} {msg} [{elapsed}]")
            .expect("valid template"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}
