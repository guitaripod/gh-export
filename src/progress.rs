use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct ProgressTracker {
    multi_progress: MultiProgress,
    main_bar: ProgressBar,
    repo_bars: Arc<Mutex<HashMap<String, ProgressBar>>>,
    total_repos: AtomicUsize,
    completed_repos: AtomicUsize,
    failed_repos: AtomicUsize,
}

impl ProgressTracker {
    pub fn new(total_repos: usize) -> Arc<Self> {
        let multi_progress = MultiProgress::new();

        let main_bar = multi_progress.add(ProgressBar::new(total_repos as u64));
        main_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} repos ({percent}%) | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        main_bar.set_message("Starting export...");

        Arc::new(Self {
            multi_progress,
            main_bar,
            repo_bars: Arc::new(Mutex::new(HashMap::new())),
            total_repos: AtomicUsize::new(total_repos),
            completed_repos: AtomicUsize::new(0),
            failed_repos: AtomicUsize::new(0),
        })
    }

    pub fn update_repo_progress(&self, repo_name: &str, current: u32, total: u32) {
        let mut bars = self.repo_bars.lock().unwrap();

        let bar = bars.entry(repo_name.to_string()).or_insert_with(|| {
            let bar = self.multi_progress.add(ProgressBar::new(total as u64));
            bar.set_style(
                ProgressStyle::default_bar()
                    .template("  └─ {msg} [{bar:30.yellow/blue}] {percent}%")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏ "),
            );
            bar.set_message(repo_name.to_string());
            bar
        });

        bar.set_position(current as u64);

        if current >= total {
            bar.finish_and_clear();
            bars.remove(repo_name);
        }
    }

    pub fn increment_completed(&self) {
        let completed = self.completed_repos.fetch_add(1, Ordering::SeqCst) + 1;
        self.main_bar.inc(1);
        self.update_main_message();

        if completed == self.total_repos.load(Ordering::SeqCst) {
            self.main_bar.finish_with_message("Export completed!");
        }
    }

    pub fn increment_failed(&self) {
        self.failed_repos.fetch_add(1, Ordering::SeqCst);
        self.main_bar.inc(1);
        self.update_main_message();
    }

    fn update_main_message(&self) {
        let completed = self.completed_repos.load(Ordering::SeqCst);
        let failed = self.failed_repos.load(Ordering::SeqCst);
        let _total = self.total_repos.load(Ordering::SeqCst);

        let msg = if failed > 0 {
            format!("Completed: {completed}, Failed: {failed}")
        } else {
            format!("Completed: {completed}")
        };

        self.main_bar.set_message(msg);
    }

    pub fn finish(&self) {
        let completed = self.completed_repos.load(Ordering::SeqCst);
        let failed = self.failed_repos.load(Ordering::SeqCst);
        let _total = self.total_repos.load(Ordering::SeqCst);

        if failed > 0 {
            self.main_bar.finish_with_message(format!(
                "Export finished with {completed} successes and {failed} failures"
            ));
        } else {
            self.main_bar.finish_with_message(format!(
                "Export completed successfully! {completed} repositories downloaded"
            ));
        }
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.total_repos.load(Ordering::SeqCst),
            self.completed_repos.load(Ordering::SeqCst),
            self.failed_repos.load(Ordering::SeqCst),
        )
    }
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}
