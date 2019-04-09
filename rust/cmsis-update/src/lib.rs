#![feature(generators, libc, proc_macro_hygiene, custom_attribute)]
/* TODO: Remove this allow and figure out why diesel's 
 * table! macro does this. This currently affets the 
 * dstore module*/
#![allow(proc_macro_derive_resolution_fallback)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_rustls;
extern crate minidom;
extern crate failure;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate slog;

extern crate utils;
extern crate pack_index;
extern crate pdsc;

use diesel::sqlite::SqliteConnection;
use hyper::{Body, Client};
use hyper::client::Connect;
use hyper_rustls::HttpsConnector;
use tokio_core::reactor::Core;
use slog::Logger;
use failure::Error;

use pack_index::config::Config;
use pdsc::Package;

pub mod upgrade;
mod redirect;
mod vidx;
mod download;
mod dl_pdsc;
mod dl_pack;
mod dstore;

use dstore::connect;
pub use dstore::{DownloadedPdsc, InstalledPack};
use dl_pdsc::{update_future};
use dl_pack::{install_future};
pub use download::DownloadProgress;

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<'a, C, I, P>(
    config: &'a Config,
    vidx_list: I,
    core: &'a mut Core,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    progress: P,
    dstore: &'a SqliteConnection,
) -> Result<Vec<DownloadedPdsc>, Error>
where
    C: Connect,
    I: IntoIterator<Item = String>,
    P: DownloadProgress + 'a,
{
    core.run(update_future(config, vidx_list, client, logger, dstore, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I, P>(
    config: &Config, vidx_list: I, logger: &Logger, progress: P
) -> Result<Vec<DownloadedPdsc>, Error>
where
    I: IntoIterator<Item = String>,
    P: DownloadProgress,
{
    let dstore = connect()?;
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    update_inner(config, vidx_list, &mut core, &client, logger, progress, &dstore)
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn install_inner<'client, 'a: 'client, C, I: 'a, P: 'client>(
    config: &'a Config,
    pdsc_list: I,
    core: &mut Core,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    progress: P,
    dstore: &'client SqliteConnection,
) -> Result<Vec<InstalledPack>, Error>
    where
    C: Connect,
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a
{
    core.run(install_future(config, pdsc_list, client, logger, dstore, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn install<'a, I: 'a, P>(
    config: &'a Config,
    pdsc_list: I,
    logger: &'a Logger,
    progress: P
) -> Result<Vec<InstalledPack>, Error>
    where
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a,
{
    let dstore = connect()?;
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    install_inner(config, pdsc_list, &mut core, &client, logger, progress, &dstore)
}
