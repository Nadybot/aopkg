CREATE TABLE IF NOT EXISTS packages
(
    `id` INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL,
    `owner` INTEGER NOT NULL,
    `name` varchar(30) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS versions
(
    `id` INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL,
    `package` INTEGER NOT NULL REFERENCES packages(`id`),
    `description` varchar(8000) NOT NULL,
    `short_description` varchar(100) NOT NULL,
    `version` varchar(12) NOT NULL,
    `author` varchar(30) NOT NULL,
    `bot_version` varchar(24) NOT NULL,
    `github` varchar(40)
);

CREATE TABLE IF NOT EXISTS requirements
(
    `id` INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL,
    `package_version` INTEGER NOT NULL REFERENCES versions(`id`),
    `name` varchar(30) NOT NULL,
    `version` varchar(12) NOT NULL
);

CREATE INDEX package_name_idx ON packages(`name`);
CREATE INDEX requirements_package_version_idx ON requirements(`package_version`);
