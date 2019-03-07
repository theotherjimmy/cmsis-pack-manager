use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use diesel::sqlite::SqliteConnection;
use failure::Error;
use futures::Stream;
use futures::prelude::{await, async_block, async_stream_block, stream_yield, Future};
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;
use std::sync::Arc;

use pack_index::config::Config;

use redirect::ClientRedirExt;

pub(crate) trait IntoDownload {
    fn into_uri(&self, &Config) -> Result<Uri, Error>;
    fn into_fd(&self, &Config) -> PathBuf;
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

fn download_file<'b,  C: Connect, P: DownloadProgress + 'b>(
    source: Uri,
    dest: PathBuf,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    spinner: Arc<P>
) -> impl Future<Item = PathBuf, Error = Error> + 'b {
    async_block!{
        if !dest.exists(){
            dest.parent().map(create_dir_all);
            let response = await!(client.redirectable(source, logger))?;
            let temp = dest.with_extension("part");
            let mut fd = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&temp)?;
            #[async]
            for bytes in response.body() {
                fd.write_all(bytes.as_ref())?;
                spinner.progress(bytes.len());
            }
            rename(&temp, &dest)?;
        }
        spinner.complete();
        Ok(dest)
    }
}

pub(crate) fn download_stream<'b, 'a: 'b, F, C, P: 'b, DL: 'a>(
    config: &'a Config,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    progress: P
) -> Box<Stream<Item = PathBuf, Error = Error> + 'b>
    where F: Stream<Item = DL, Error = Error> + 'b,
          C: Connect,
          DL: IntoDownload,
          P: DownloadProgress
{
    Box::new(
        async_stream_block!(
            let to_dl = await!(stream.collect())?;
            let len = to_dl.iter().count();
            progress.size(len);
            for from in to_dl {
                let dest = from.into_fd(config);
                let source = from.into_uri(config)?;
                let new_prog = Arc::new(progress.for_file(&dest.to_string_lossy()));
                stream_yield!(download_file(source.clone(), dest, client, logger, new_prog.clone())
                              .map(Some)
                              .or_else(
                                  move |e| {
                                      slog_error!(logger, "download of {:?} failed: {}", source, e);
                                      new_prog.complete();
                                      Ok(None)
                                  }))
            }
            Ok(())
        ).buffer_unordered(32).filter_map(|x| x)
    )
}

pub(crate) trait DownloadToDatabase {
    fn into_uri(&self, &Config) -> Result<Uri, Error>;
    fn in_database(&self, &SqliteConnection) -> bool;
    fn insert_text(&mut self, String, &SqliteConnection) -> Result<(), Error>;
}

fn download_to_db<'a,  C: Connect, P: DownloadProgress + 'a, D: DownloadToDatabase + 'a>(
    source: D,
    dstore: &'a SqliteConnection,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    config: &'a Config,
) -> impl Future<Item = (), Error = Error> + 'a {
    async_block!{
        if !source.in_database(dstore){
	    let uri = source.into_uri(config)?;
            let response = await!(client.redirectable(uri, logger))?;
            let chunk = await!(response.body().concat2())?;
	    let text = String::from_utf8(chunk.to_vec())?;
            source.insert_text(text, dstore);
        }
        Ok(())
    }
}


pub(crate) fn download_stream_to_db<'b, 'a: 'b, F, C, P: 'b, DL: 'a>(
    config: &'a Config,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    progress: P,
    dstore: &'b SqliteConnection,
) -> Box<Stream<Item = (), Error = Error> + 'b>
    where F: Stream<Item = DL, Error = Error> + 'b,
          C: Connect,
          DL: DownloadToDatabase,
          P: DownloadProgress
{
    Box::new(
        async_stream_block!(
            let to_dl = await!(stream.collect())?;
            let len = to_dl.iter().count();
            progress.size(len);
            for from in to_dl {
		// TODO: This is not a good replacement
                let left = Arc::new(progress.for_file(""));
		let right = left.clone();
                stream_yield!(download_to_db(from, dstore, client, logger, config)
                     .map(|x| {
		         left.complete();
		         Some(x)
		     })
                     .or_else( move |e| {
                         slog_error!(logger, "download of ? failed: {}", e);
                         right.complete();
                         Ok(None)
                     }));
            }
            Ok(())
        ).buffer_unordered(32).filter_map(|x| x)
    )
}
