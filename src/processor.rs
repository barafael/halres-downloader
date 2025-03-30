use chrono::NaiveDate;
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::Response;
use select::{
    document::Document,
    predicate::{Attr, Name},
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::Resource;

fn extract_title_and_content(document: &Document) -> (Option<String>, Option<&str>) {
    let mut title = None;
    let mut content = None;

    if let Some(the_title) = document.find(Name("title")).next() {
        title = Some(the_title.text());
    }
    if let Some(description) = document.find(Attr("name", "description")).next() {
        content = description.attr("content");
    }
    (title, content)
}

async fn page_details(
    (response, timestamp): (Response, NaiveDate),
) -> Result<Resource, reqwest::Error> {
    let url = response.url().clone();
    let text = response.text().await?;
    let document = Document::from(text.as_str());
    let (title, content) = extract_title_and_content(&document);
    let resource = Resource {
        url,
        content: content.unwrap_or_default().to_string(),
        title: title.unwrap_or_default(),
        timestamp,
    };
    Ok(resource)
}

pub async fn processor(
    mut pages: mpsc::Receiver<(Response, NaiveDate)>,
    forward: mpsc::Sender<Resource>,
) {
    let mut work = FuturesUnordered::new();

    loop {
        tokio::select! {
            Some(page) = pages.recv() => {
                work.push(page_details(page));
            },
            Some(resource) = work.next() => {
                match resource {
                    Ok(resource) => {
                        if let Err(error) = forward.send(resource).await {
                            warn!(%error, "Page processor cannot forward, shutting down");
                            break;
                        }
                    }
                    Err(error) => {
                        warn!(%error, "Failed to process page");
                    }
                }
            },
            else => { break }
        }
    }
    info!("Page processor finished");
}
