use clap::Parser;
use rockbot_cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(rockbot_cli::run(cli))
}
