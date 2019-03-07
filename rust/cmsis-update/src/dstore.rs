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
    url: &'a str,
    vendor: &'a str,
    name: &'a str,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_meta: &'a str,
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
	    version_meta: &pdsc.version,
	}
    }
}

fn insert_pdscref(pdsc: &PdscRef, dstore: &SqliteConnection) -> QueryResult<usize> 
{
    diesel::insert_into(current_pdsc::table)
        .values(InsertablePdscRef::from_pdscref(pdsc))
        .execute(dstore)
}

#[derive(Debug)]
#[derive(Queryable)]
#[derive(Identifiable)]
#[derive(AsChangeset)]
#[table_name="current_pdsc"]
pub struct InsertedPdsc {
    id: usize,
    pub(crate) url: String,
    pub(crate) vendor: String,
    pub(crate) name: String,
    version_major: i32,
    version_minor: i32,
    version_patch: i32,
    version_meta: String,
    parsed: bool,
    pdsc_text: Option<String>,
}

impl InsertedPdsc {
    pub fn try_from_ref(pdsc: PdscRef, dstore: &SqliteConnection) -> QueryResult<Self> {
        let id = insert_pdscref(&pdsc, dstore)?;
	Ok(Self {
	    id,
	    url: pdsc.url,
	    vendor: pdsc.vendor,
	    name: pdsc.name,
	    version_major: 0,
	    version_minor: 0,
	    version_patch: 0,
	    version_meta: pdsc.version,
	    parsed: false,
	    pdsc_text: None,
	})
    }

    pub fn insert_text(mut self, new_text: String, dstore: &SqliteConnection) -> QueryResult<Self> {
        use dstore::current_pdsc::dsl::*;
        self.pdsc_text.replace(new_text);
	diesel::update(current_pdsc).set(&self).execute(dstore)?;
	Ok(self)
    }
}

