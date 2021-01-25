ALTER TABLE versions RENAME TO old_versions;

CREATE TABLE IF NOT EXISTS versions
(
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "package" INTEGER REFERENCES packages("id") NOT NULL,
    "description" varchar(8000) NOT NULL,
    "short_description" varchar(100) NOT NULL,
    "version" varchar(12) NOT NULL COLLATE semver_collation,
    "author" varchar(30) NOT NULL,
    "bot_type" varchar(15) NOT NULL,
    "bot_version" varchar(24) NOT NULL
);

INSERT INTO versions SELECT * FROM old_versions;

DROP TABLE old_versions;
