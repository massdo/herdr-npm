use herdr_npm::adapters::env;
use herdr_npm::adapters::herdr_socket::HerdrSocket;
use herdr_npm::adapters::tui;
use herdr_npm::application::open_sidebar::open_empty_sidebar;
use herdr_npm::domain::error::AppError;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}

fn run() -> Result<(), AppError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--toggle") => toggle(),
        Some("--help" | "-h") => {
            println!("herdr-npm [--toggle]");
            Ok(())
        }
        Some(other) => Err(AppError::Io {
            message: format!("unknown argument: {other}"),
        }),
        None => tui_mode(),
    }
}

fn toggle() -> Result<(), AppError> {
    let process = env::load()?;
    let origin = env::origin_from_env()?;
    let herdr = HerdrSocket::new(process.socket_path);
    open_empty_sidebar(&herdr, &origin)?;
    Ok(())
}

fn tui_mode() -> Result<(), AppError> {
    let process = env::load()?;
    tui::run(process)
}
