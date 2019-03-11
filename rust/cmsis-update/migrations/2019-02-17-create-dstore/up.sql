CREATE TABLE current_pdsc (
	vendor VARCHAR NOT NULL,
	name VARCHAR NOT NULL,
	version_major INTEGER NOT NULL,
	version_minor INTEGER NOT NULL,
	version_patch INTEGER NOT NULL,
	version_full VARCHAR NOT NULL,
	url VARCHAR NOT NULL,
	pdsc_text VARCHAR,
	parsed BOOL NOT NULL,
	UNIQUE (vendor, name, version_full)
);

