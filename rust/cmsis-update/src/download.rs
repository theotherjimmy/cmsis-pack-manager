use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use diesel::sqlite::SqliteConnection;
use failure::Error;
use futures::Stream;
use futures::prelude::{await, async_block, async_stream_block, stream_yield, Future};
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::config::Config;

use redirect::ClientRedirExt;

pub(crate) trait StartDownload {
    type DL: IntoDownload;
    fn start_download(self, &SqliteConnection) -> Result<Self::DL, Error>;
}

pub(crate) trait IntoDownload {
    type Done;
    fn into_uri(&self, &Config) -> Result<Uri, Error>;
    fn into_fd(&self, &Config) -> PathBuf;
    fn insert_downloaded(self, &Path, &SqliteConnection) -> Result<Self::Done, Error>;
}

pub trait DownloadProgress: Send {
    fn size(&self, files: usize);
    fn progress(&self, bytes: usize);
    fn complete(&self);
    fn for_file(&self, file: &str) -> Self;
}

impl DownloadProgress for () {
    fn size(&self, _: usize) {}
    fn progress(&self, _: usize) {}
    fn complete(&self) {}
    fn for_file(&self, _: &str) -> Self {
        ()
    }
}

fn download_to_db<'a,  C: Connect, D: IntoDownload + 'a>(
    source: D,
    dstore: &'a SqliteConnection,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    config: &'a Config,
) -> impl Future<Item = D::Done, Error = Error> + 'a {
    async_block!{
        let dest = source.into_fd(config);
        if !dest.exists(){
            dest.parent().map(create_dir_all);
            let url = source.into_uri(config)?;
            let response = await!(client.redirectable(url, logger))?;
            let temp = dest.with_extension("part");
            let mut fd = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&temp)?;
            #[async]
            for bytes in response.body() {
                fd.write_all(bytes.as_ref())?;
            }
            rename(&temp, &dest)?;
        }
        Ok(source.insert_downloaded(&dest, dstore)?)
    }
}


pub(crate) fn download_stream_to_db<'a, F, C, P, S>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    dstore: &'a SqliteConnection,
    progress: P
) -> Box<Stream<Item = <S::DL as IntoDownload>::Done, Error = Error> + 'a>
    where F: Stream<Item = S, Error = Error> + 'a,
          C: Connect,
          S: StartDownload + 'a,
          P: DownloadProgress + 'a
{
    Box::new(
        async_stream_block!{
            let to_dl = await!(stream.collect())?;
            let to_dl: Vec<Result<S::DL, Error>> = to_dl.into_iter().map(|s| s.start_download(dstore)).collect();
            let len = to_dl.iter().count();
            progress.size(len);
            for from in to_dl {
                let from = from?;
                let for_file = progress.for_file(
                    &from.into_fd(config).to_string_lossy()
                );
                stream_yield!(
                    download_to_db(from, dstore, client, logger, config)
                    .then(move |res|{
                        for_file.complete();
                        Ok(res.ok())
                    })
                )
            }
            Ok(())
        }.buffer_unordered(32).filter_map(|x| x)
    )
}
