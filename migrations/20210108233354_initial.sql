CREATE TABLE IF NOT EXISTS packages
(
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "owner" INTEGER NOT NULL,
    "name" varchar(30) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS versions
(
    "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    "package" INTEGER REFERENCES packages("id") NOT NULL,
    "description" varchar(8000) NOT NULL,
    "short_description" varchar(100) NOT NULL,
    "version" varchar(12) NOT NULL,
    "author" varchar(30) NOT NULL,
    "bot_type" varchar(15) NOT NULL,
    "bot_version" varchar(24) NOT NULL
);

CREATE INDEX package_name_idx ON packages("name");
