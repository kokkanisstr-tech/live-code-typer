#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--migrate-data")
    {
        let library = live_code_typer::storage::load()?;
        live_code_typer::storage::save(&library)?;
        return Ok(());
    }

    live_code_typer::ui::run()?;
    Ok(())
}
