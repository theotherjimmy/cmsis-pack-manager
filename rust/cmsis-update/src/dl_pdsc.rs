use std::path::{Path, PathBuf};

use diesel::sqlite::SqliteConnection;
use failure::Error;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::config::Config;

use dstore::{InsertedPdsc, DownloadedPdsc};
use download::{StartDownload, IntoDownload, DownloadProgress, download_stream_to_db};
use vidx::{download_vidx_list, flatmap_pdscs};

impl StartDownload for InsertedPdsc {
    type DL = Self;
    fn start_download(self, _: &SqliteConnection) -> Result<InsertedPdsc, Error> {
        Ok(self)
    }
}

impl IntoDownload for InsertedPdsc {
    type Done = DownloadedPdsc;
    fn into_uri(&self, _: &Config) -> Result<Uri, Error> {
        let &InsertedPdsc {ref url, ref vendor, ref name, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd(&self, config: &Config) -> PathBuf {
        let &InsertedPdsc {ref version_full, ref vendor, ref name, ..} = self;
        let mut filename = config.pack_store.clone();
        let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version_full);
        filename.push(pdscname);
        filename
    }

    fn insert_downloaded(self, path: &Path, dstore: &SqliteConnection) -> Result<DownloadedPdsc, Error> {
        //TODO: maybe don't clone the path string here
        Ok(self.insert_path(path.to_string_lossy().to_string(), dstore)?)
    }
}

/// Create a future of the update command.
pub fn update_future<'a, C, I, P>(
    config: &'a Config,
    vidx_list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    dstore: &'a SqliteConnection,
    progress: P,
) -> impl Future<Item = Vec<DownloadedPdsc>, Error = Error> + 'a
    where C: Connect,
          I: IntoIterator<Item = String> + 'a,
	  P: DownloadProgress + 'a
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .filter_map(move |vidx| match vidx {
            Ok(v) => Some(flatmap_pdscs(v, client, logger, dstore)),
            Err(_) => None,
        })
        .flatten();
    download_stream_to_db(config, pdsc_list, client, logger, dstore, progress).collect()
}
