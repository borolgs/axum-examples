use lazy_static::lazy_static;
use rusqlite_migration::{Migrations, M};

lazy_static! {
    pub static ref MIGRATIONS: Migrations<'static> = Migrations::new(vec![
        M::up(r#"
                CREATE TABLE users (
                    id BLOB PRIMARY KEY CHECK(length(id) = 16) NOT NULL UNIQUE DEFAULT (uuid7_now()),
                    
                    email TEXT NOT NULL UNIQUE,
                    role TEXT NOT NULL DEFAULT 'member', -- admin | member | guest
                    oauth_provider TEXT,
                    access_token TEXT,
                    password TEXT,

                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    created_by BLOB CHECK(length(created_by) = 16),
                    updated_at DATETIME,
                    updated_by BLOB CHECK(length(updated_by) = 16),
                    
                    FOREIGN KEY (created_by) REFERENCES users(id),
                    FOREIGN KEY (updated_by) REFERENCES users(id)
                );
                CREATE TABLE notes (
                    id BLOB PRIMARY KEY CHECK(length(id) = 16) NOT NULL UNIQUE DEFAULT (uuid7_now()),
                    
                    title TEXT,
                    text TEXT,

                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    created_by BLOB CHECK(length(created_by) = 16),
                    updated_at DATETIME,
                    updated_by BLOB CHECK(length(updated_by) = 16),

                    FOREIGN KEY (created_by) REFERENCES users (id),
                    FOREIGN KEY (updated_by) REFERENCES users (id)
                );
                "#
        ),
        // seed
        M::up(
            r#"INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6138-5b4f-722d-97c5-29b927cedbd4'), 'first', '1');
            INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6146-32f4-7f98-90b8-19fda2c87491'), 'second', '2');
            INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6146-32f4-7948-8289-cfb5cdb2b2af'), 'third', '3');"#
        ),
    ]);
}
