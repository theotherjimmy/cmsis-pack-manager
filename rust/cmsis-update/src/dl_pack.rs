use std::path::{Path, PathBuf};

use diesel::sqlite::SqliteConnection;
use failure::Error;
use futures::stream::iter_ok;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pdsc::Package;
use pack_index::config::Config;

use download::{StartDownload, IntoDownload, DownloadProgress, download_stream_to_db};
use dstore::{InsertedPack, InstalledPack};

impl<'a> StartDownload for &'a Package {
    type DL = InsertedPack;
    fn start_download(self, dstore: &SqliteConnection) -> Result<InsertedPack, Error> {
        Ok(InsertedPack::try_from_package(self, dstore)?)
    }
}

impl IntoDownload for InsertedPack {
    type Done = InstalledPack;
    fn into_uri(&self, _: &Config) -> Result<Uri, Error> {
        let &InsertedPack{ref name, ref vendor, ref url, ref version_full, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.{}.pack", url, vendor, name, version_full)
        } else {
            format!("{}/{}.{}.{}.pdsc", url, vendor, name, version_full)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd(&self, config: &Config) -> PathBuf {
        let &InsertedPack{ref name, ref vendor, ref version_full, ..} = self;
        let mut filename = config.pack_store.clone();
        filename.push(Path::new(vendor));
        filename.push(Path::new(name));
        filename.push(format!("{}.pack", version_full));
        filename
    }

    fn insert_downloaded(self, path: &Path, dstore: &SqliteConnection) -> Result<InstalledPack, Error>{
        //TODO: maybe don't clone the path here
        Ok(self.insert_path(path.to_string_lossy().to_string(), dstore)?)
    }
}


pub fn install_future<'client,'a: 'client,  C, I, P>(
    config: &'a Config,
    pdscs: I,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    dstore: &'client SqliteConnection,
    progress: P,
) -> impl Future<Item = Vec<InstalledPack>, Error = Error> + 'client
    where C: Connect,
          I: IntoIterator<Item = &'a Package> + 'a,
          P: DownloadProgress + 'client,
{
    download_stream_to_db(config, iter_ok(pdscs), client, logger, dstore, progress).collect()
}
