use diesel::sqlite::SqliteConnection;
use failure::Error;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::config::Config;

use dstore::InsertedPdsc;
use download::{DownloadToDatabase, DownloadProgress, download_stream_to_db};
use vidx::{download_vidx_list, flatmap_pdscs};

impl DownloadToDatabase for InsertedPdsc {
    fn into_uri(&self, _: &Config) -> Result<Uri, Error> {
        let &InsertedPdsc {ref url, ref vendor, ref name, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }.parse()?;
        Ok(uri)
    }

    fn in_database(&self, _dstore: &SqliteConnection) -> bool {
        false
    }

    fn insert_text(self, text: String, dstore: &SqliteConnection) -> Result<Self, Error> {
        Ok(self.insert_text(text, dstore)?)
    }
}

/// Create a future of the update command.
pub fn update_future<'a, C, I>(
    config: &'a Config,
    vidx_list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    dstore: &'a SqliteConnection,
) -> impl Future<Item = Vec<InsertedPdsc>, Error = Error> + 'a
    where C: Connect,
          I: IntoIterator<Item = String> + 'a,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .filter_map(move |vidx| match vidx {
            Ok(v) => Some(flatmap_pdscs(v, client, logger, dstore)),
            Err(_) => None,
        })
        .flatten();
    download_stream_to_db(config, pdsc_list, client, logger, dstore).collect()
}
