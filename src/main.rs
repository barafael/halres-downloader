use std::{fs::File, path::PathBuf, time::Instant};

use clap::Parser;
use halres_downloader::download_pages;
use tracing::{debug, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
struct Arguments {
    /// Input file.
    #[arg(short, long, default_value = "urls.csv")]
    file: PathBuf,

    /// Output file.
    #[arg(short, long, default_value = "resources.json")]
    output: PathBuf,

    /// Channel size.
    #[arg(long, default_value_t = 64)]
    channel_size: usize,

    /// Maximum number of concurrent operations.
    #[arg(long, default_value_t = 64)]
    concurrency_limit: usize,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let start = Instant::now();
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    let args = Arguments::parse();

    let file = File::open(&args.file).expect("Failed to open file");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_reader(file);

    let mut records = Vec::new();
    for record in reader.deserialize() {
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                warn!(%error, "Failed to parse record");
                continue;
            }
        };
        debug!(?record, "Parsed record");
        records.push(record);
    }

    let resources = download_pages(args.channel_size, args.concurrency_limit, records).await;

    println!("Time elapsed: {:?}s", start.elapsed().as_secs_f32());

    let json = serde_json::to_string_pretty(&resources).expect("Failed to serialize resources");
    std::fs::write(args.output, json).unwrap();
}
