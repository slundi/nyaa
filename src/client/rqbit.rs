use std::error::Error;

use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use urlencoding::encode;

use crate::{source::Item, util::conv::add_protocol};

use super::{
    multidownload, BatchDownloadResult, ClientConfig, DownloadClient, DownloadError,
    SingleDownloadResult,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct RqbitConfig {
    pub base_url: String,
    pub use_magnet: Option<bool>,
    pub overwrite: Option<bool>,
    pub output_folder: Option<String>,
    pub yank_full_magnet: Option<bool>,
}

pub struct RqbitClient;

#[derive(Serialize, Deserialize, Clone)]
pub struct RqbitForm {
    pub overwrite: Option<bool>,
    pub output_folder: Option<String>,
}

impl Default for RqbitConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3030".to_owned(),
            use_magnet: None,
            overwrite: None,
            output_folder: None,
            yank_full_magnet: None,
        }
    }
}

async fn add_torrent(
    conf: &RqbitConfig,
    link: String,
    client: &reqwest::Client,
) -> Result<Response, Box<dyn Error + Send + Sync>> {
    let base_url = add_protocol(conf.base_url.clone(), false)?;
    let mut url = base_url.join("/torrents")?;
    let mut query: Vec<String> = vec![];
    if let Some(ow) = conf.overwrite {
        query.push(format!("overwrite={}", ow));
    }
    if let Some(out) = conf.output_folder.as_ref() {
        query.push(format!(
            "output_folder={}",
            encode(&shellexpand::tilde(out))
        ));
    }
    url.set_query(Some(&query.join("&")));

    match client.post(url).body(link).send().await {
        Ok(res) => Ok(res),
        Err(e) => Err(e.into()),
    }
}

impl DownloadClient for RqbitClient {
    async fn download(
        item: Item,
        conf: ClientConfig,
        client: reqwest::Client,
    ) -> SingleDownloadResult {
        let conf = match conf.rqbit.clone() {
            Some(q) => q,
            None => {
                return SingleDownloadResult::error("Failed to get rqbit config");
            }
        };
        let link = super::Client::get_link(
            conf.use_magnet.unwrap_or(true),
            conf.yank_full_magnet,
            item.torrent_link.clone(),
            item.magnet_link.clone(),
        );
        let res = match add_torrent(&conf, link, &client).await {
            Ok(r) => r,
            Err(e) => {
                return SingleDownloadResult::error(DownloadError(format!(
                    "Failed to get response from rqbit\n{}",
                    e
                )));
            }
        };
        if res.status() != StatusCode::OK {
            return SingleDownloadResult::error(DownloadError(format!(
                "rqbit returned status code {}",
                res.status().as_u16()
            )));
        }

        SingleDownloadResult::success("Successfully sent torrent to rqbit".to_owned(), item.id)
    }

    async fn batch_download(
        items: Vec<Item>,
        conf: ClientConfig,
        client: reqwest::Client,
    ) -> BatchDownloadResult {
        multidownload::<RqbitClient, _>(
            |s| format!("Successfully sent {} torrents to rqbit", s),
            &items,
            &conf,
            &client,
        )
        .await
    }

    fn load_config(cfg: &mut ClientConfig) {
        if cfg.rqbit.is_none() {
            cfg.rqbit = Some(RqbitConfig::default());
        }
    }
}
