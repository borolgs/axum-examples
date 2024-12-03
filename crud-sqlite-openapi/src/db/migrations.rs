use lazy_static::lazy_static;
use rusqlite_migration::{Migrations, M};

lazy_static! {
    static ref DEV_FIXTURES: String = _dev_fixtures();
    pub static ref MIGRATIONS: Migrations<'static> = Migrations::new(vec![
        M::up(
            r#"
            CREATE TABLE users (
                id BLOB PRIMARY KEY CHECK(length(id) = 16) NOT NULL UNIQUE DEFAULT (uuid7_now()),
                email TEXT NOT NULL UNIQUE,

                role TEXT NOT NULL DEFAULT 'member', -- admin | member | guest
                status TEXT NOT NULL, -- active | pending | blocked
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
        "#
        ),
        M::up(
            r#"
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
        M::up(&DEV_FIXTURES),
    ]);
}

fn _dev_fixtures() -> String {
    let user_id = "018f6146-32f4-7948-8289-cfb5cdb2b2af";
    format!(
        r#"
        INSERT INTO users (id, email, role, status) VALUES (uuid_blob('{user_id}'), 'fake@mail.com', 'admin', 'active');
        "#
    )
}
