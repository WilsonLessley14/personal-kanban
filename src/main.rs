#[cfg(unix)]
fn setup_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn setup_sigpipe() {}

fn main() {
    setup_sigpipe();
    personal_kanban::cli::run_with_exit();
}
