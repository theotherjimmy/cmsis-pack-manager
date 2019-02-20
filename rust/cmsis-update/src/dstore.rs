use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use failure::Error;

use pack_index::PdscRef;

embed_migrations!();

table!{
    use diesel::sql_types::*;
    current_pdsc {
	id -> Integer,
	vendor -> VarChar,
	name -> VarChar,
	version_major -> Integer,
	version_minor -> Integer,
	version_patch -> Integer,
	version_meta -> Nullable<VarChar>,
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
    pub url: &'a str,
    pub vendor: &'a str,
    pub name: &'a str,
    pub version_major: i32,
    pub version_minor: i32,
    pub version_patch: i32,
    pub version_meta: &'a str,
    pub parsed: bool,
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
	    version_meta: &pdsc.version,
	}
    }
}

pub(crate) fn insert_pdscref(pdsc: &PdscRef, dstore: &SqliteConnection) -> QueryResult<usize> 
{
    diesel::insert_into(current_pdsc::table)
        .values(InsertablePdscRef::from_pdscref(pdsc))
        .execute(dstore)
}
