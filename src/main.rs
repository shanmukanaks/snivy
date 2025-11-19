use clap::Parser;
use snivy::{App, AppResult, Settings, telemetry};

#[derive(Debug, Parser)]
#[command(version, about = "Snivy trading system")]
struct Cli {
    #[arg(short, long, default_value = "configs/default.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();
    let settings = Settings::load_from(&cli.config)?;
    telemetry::init(&settings.telemetry)?;
    let app = App::new(settings);
    app.run().await
}
