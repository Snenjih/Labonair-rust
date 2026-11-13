//! `labonair-host`: the detached session host daemon (ADR 0009).
//!
//! Usage: `labonair-host --data-dir <path>`. Prints nothing; logs to
//! `<data-dir>/host/host.log`.

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    use labonair_terminal_host::server::{
        ensure_private_dir, ExitReason, HostPaths, Server, ServerConfig, ServerError,
    };
    use std::process::ExitCode;

    let mut args = std::env::args().skip(1);
    let mut data_dir = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = args.next(),
            _ => return ExitCode::from(2),
        }
    }
    let Some(data_dir) = data_dir.map(std::path::PathBuf::from) else {
        return ExitCode::from(2);
    };

    // The spawner normally calls setsid(); be robust when run directly.
    unsafe {
        libc::setsid();
    }
    let paths = HostPaths::new(&data_dir);
    if ensure_private_dir(&paths.dir).is_err() {
        return ExitCode::from(1);
    }
    redirect_stdio(&paths.log);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log);
    if let Ok(log) = log {
        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(log))
            .with_ansi(false)
            .init();
    }

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "cannot start runtime");
            return ExitCode::from(1);
        }
    };
    runtime.block_on(async move {
        let server = match Server::bind(ServerConfig::new(data_dir)) {
            Ok(server) => server,
            Err(ServerError::AlreadyRunning) => return ExitCode::SUCCESS,
            Err(error) => {
                tracing::error!(%error, "cannot start host");
                return ExitCode::from(1);
            }
        };
        let handle = server.handle();
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            let (Ok(mut term), Ok(mut hup)) = (
                signal(SignalKind::terminate()),
                signal(SignalKind::hangup()),
            ) else {
                return;
            };
            tokio::select! {
                _ = term.recv() => {}
                _ = hup.recv() => {}
            }
            handle.stop(ExitReason::Signal);
        });
        tracing::info!(pid = std::process::id(), "labonair-host started");
        let reason = server.run().await;
        tracing::info!(?reason, "labonair-host exiting");
        ExitCode::SUCCESS
    })
}

/// stdin from /dev/null; stdout and stderr appended to the log file.
#[cfg(unix)]
fn redirect_stdio(log: &std::path::Path) {
    use std::os::unix::io::AsRawFd;
    let null = std::fs::File::open("/dev/null");
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log);
    unsafe {
        if let Ok(null) = &null {
            libc::dup2(null.as_raw_fd(), 0);
        }
        if let Ok(log) = &log {
            libc::dup2(log.as_raw_fd(), 1);
            libc::dup2(log.as_raw_fd(), 2);
        }
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("labonair-host is not supported on this platform");
}
