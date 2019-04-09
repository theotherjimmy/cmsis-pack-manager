use std::path::PathBuf;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use failure::Error;

use pack_index::PdscRef;
use pdsc::Package;

embed_migrations!();

table!{
    use diesel::sql_types::*;
    current_pdsc (vendor, name, version_full) {
        vendor -> VarChar,
        name -> VarChar,
        version_major -> Integer,
        version_minor -> Integer,
        version_patch -> Integer,
        version_full -> VarChar,
        url -> VarChar,
        parsed -> Bool,
        path -> Nullable<VarChar>,
    }
}

pub fn connect() -> Result<SqliteConnection, Error> {
    let database_url = "index.sqlite";
    let connection = SqliteConnection::establish(database_url)?;
    embedded_migrations::run(&connection)?;	
    Ok(connection)
}

#[derive(Debug, Insertable)]
#[table_name="current_pdsc"]
struct InsertablePdscRef<'a> {
    url: &'a str,
    vendor: &'a str,
    name: &'a str,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_full: &'a str,
    parsed: bool,
}

impl<'a> InsertablePdscRef<'a> {
    fn from_pdscref(pdsc: &'a PdscRef) -> Self {
        Self {
	    parsed: false,
	    url: &pdsc.url,
	    name: &pdsc.name,
	    vendor: &pdsc.vendor,
	    version_major: 0,
	    version_minor: 0,
	    version_patch: 0,
	    version_full: &pdsc.version,
	}
    }
}

fn insert_pdscref(pdsc: &PdscRef, dstore: &SqliteConnection) -> QueryResult<usize> 
{
    diesel::insert_or_ignore_into(current_pdsc::table)
        .values(InsertablePdscRef::from_pdscref(pdsc))
        .execute(dstore)
}

#[derive(Debug)]
#[derive(Queryable)]
#[derive(Identifiable)]
#[table_name="current_pdsc"]
#[primary_key(vendor, name, version_full)]
pub struct InsertedPdsc {
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    pub(crate) version_full: String,
    pub(crate) url: String,
    parsed: bool,
    path: Option<String>,
}

impl InsertedPdsc {
    pub fn try_from_ref(pdsc: PdscRef, dstore: &SqliteConnection) -> QueryResult<Self> {
        insert_pdscref(&pdsc, dstore)?;
        current_pdsc::table.find((pdsc.vendor, pdsc.name, pdsc.version))
            .get_result(dstore)
    }

    pub fn insert_path(self, new_text: String, dstore: &SqliteConnection) -> QueryResult<DownloadedPdsc> {
        diesel::update(&self).set(current_pdsc::dsl::path.eq(new_text)).execute(dstore)?;
        current_pdsc::table.find((self.vendor, self.name, self.version_full))
            .get_result(dstore)
    }

}

#[derive(Debug)]
#[derive(Queryable)]
#[derive(Identifiable)]
#[table_name="current_pdsc"]
#[primary_key(vendor, name, version_full)]
pub struct DownloadedPdsc {
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    pub(crate) version_full: String,
    pub(crate) url: String,
    parsed: bool,
    path: Option<String>,
}

impl DownloadedPdsc {
    pub fn into_pathbuf(self) -> PathBuf {
        self.path.unwrap().into()
    }
}

table!{
    use diesel::sql_types::*;
    installed_packs (vendor, name, version_full) {
        vendor -> VarChar,
        name -> VarChar,
        version_major -> Integer,
        version_minor -> Integer,
        version_patch -> Integer,
        version_full -> VarChar,
        url -> VarChar,
        path -> Nullable<VarChar>,
    }
}

#[derive(Debug, Insertable)]
#[table_name="installed_packs"]
struct InsertablePack<'a> {
    url: &'a str,
    vendor: &'a str,
    name: &'a str,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_full: &'a str,
}

impl<'a> InsertablePack<'a> {
    fn from_pack(package: &'a Package, version: &'a str) -> Self {
        Self {
            url: &package.url,
            name: &package.name,
            vendor: &package.vendor,
            version_major: 0,
            version_minor: 0,
            version_patch: 0,
            version_full: version,
        }
    }

    fn insert(&self, dstore: &SqliteConnection) -> QueryResult<usize>{
        diesel::insert_or_ignore_into(installed_packs::table)
            .values(self)
            .execute(dstore)
    }
}


#[derive(Debug)]
#[derive(Queryable)]
#[derive(Identifiable)]
#[derive(AsChangeset)]
#[table_name="installed_packs"]
#[primary_key(vendor, name, version_full)]
pub struct InsertedPack {
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    pub(crate) version_full: String,
    pub(crate) url: String,
    path: Option<String>,
}

impl InsertedPack {
    pub fn try_from_package(pack: &Package, dstore: &SqliteConnection) -> QueryResult<Self> {
        let version: &str = pack.releases.latest_release().version.as_ref();
        InsertablePack::from_pack(pack, version).insert(dstore)?;
        installed_packs::table.find((&pack.vendor, &pack.name, version))
            .get_result(dstore)
    }

    pub fn insert_path(self, new_text: String, dstore: &SqliteConnection) -> QueryResult<InstalledPack> {
        diesel::update(&self).set(installed_packs::dsl::path.eq(new_text)).execute(dstore)?;
        installed_packs::table.find((self.vendor, self.name, self.version_full))
            .get_result(dstore)
    }
}

#[derive(Debug)]
#[derive(Queryable)]
#[derive(Identifiable)]
#[table_name="installed_packs"]
#[primary_key(vendor, name, version_full)]
pub struct InstalledPack {
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_full: String,
    pub(crate) url: String,
    path: Option<String>,
}

impl InstalledPack {
    pub fn into_pathbuf(self) -> PathBuf {
        self.path.unwrap().into()
    }
}
