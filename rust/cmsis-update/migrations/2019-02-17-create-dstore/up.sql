CREATE TABLE current_pdsc (
	id SERIAL PRIMARY KEY,
	vendor VARCHAR NOT NULL,
	name VARCHAR NOT NULL,
	version_major INTEGER NOT NULL,
	version_minor INTEGER NOT NULL,
	version_patch INTEGER NOT NULL,
	version_meta INTEGER NOT NULL,
	url VARCHAR NOT NULL,
	pdsc_text VARCHAR,
	parsed BOOL NOT NULL
);
