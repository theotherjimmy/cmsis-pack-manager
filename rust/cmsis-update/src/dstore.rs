use std::path::PathBuf;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use failure::Error;

use pack_index::PdscRef;

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
	pdsc_text -> Nullable<VarChar>,
	parsed -> Bool,
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
#[derive(AsChangeset)]
#[table_name="current_pdsc"]
#[primary_key(vendor, name, version_full)]
pub struct InsertedPdsc {
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_full: String,
    pub(crate) url: String,
    pub(crate) pdsc_text: Option<String>,
    parsed: bool,
}

impl InsertedPdsc {
    pub fn try_from_ref(pdsc: PdscRef, dstore: &SqliteConnection) -> QueryResult<Self> {
        insert_pdscref(&pdsc, dstore)?;
	current_pdsc::table.find((pdsc.vendor, pdsc.name, pdsc.version))
	    .get_result(dstore)
    }

    pub fn insert_text(mut self, new_text: String, dstore: &SqliteConnection) -> QueryResult<Self> {
        self.pdsc_text.replace(new_text);
	diesel::update(&self).set(&self).execute(dstore)?;
	Ok(self)
    }

    pub fn into_pathbuf(self) -> Option<PathBuf> {
        self.pdsc_text.map(PathBuf::from)
    }
}

