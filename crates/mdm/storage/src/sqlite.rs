//! SQLite storage implementation.

use color_eyre::eyre::WrapErr as _;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

use crate::models::*;
use crate::schema::*;
use crate::traits::*;
use mdm_core::{EnrollId, PushInfo, QueuedCommand};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

/// SQLite-based storage.
#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage from a database URL.
    pub fn new(database_url: &str) -> color_eyre::eyre::Result<Self> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .wrap_err("failed to create connection pool")?;

        Ok(Self { pool })
    }

    /// Run migrations.
    pub fn run_migrations(&self) -> color_eyre::eyre::Result<()> {
        use diesel_migrations::MigrationHarness as _;

        let mut conn = self
            .pool
            .get()
            .wrap_err("failed to get connection for migrations")?;

        conn.run_pending_migrations(crate::MIGRATIONS)
            .map_err(|e| color_eyre::eyre::eyre!("migration failed: {}", e))?;

        Ok(())
    }

    fn conn(
        &self,
    ) -> color_eyre::eyre::Result<diesel::r2d2::PooledConnection<ConnectionManager<SqliteConnection>>>
    {
        self.pool
            .get()
            .wrap_err("failed to get database connection")
    }
}

impl CheckinStore for SqliteStorage {
    fn store_authenticate(
        &self,
        id: &EnrollId,
        msg: &mdm_core::Authenticate,
    ) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;
        let now = chrono::Utc::now().naive_utc();

        // Clear command queue first
        diesel::delete(commands::table.filter(commands::enrollment_id.eq(&id.id)))
            .execute(&mut conn)
            .wrap_err("failed to clear command queue")?;

        // Upsert enrollment (disabled until TokenUpdate)
        let new_enrollment = NewEnrollment {
            id: &id.id,
            enroll_type: &format!("{:?}", id.enroll_type),
            device_id: Some(&id.id),
            parent_id: id.parent_id.as_deref(),
            topic: &msg.topic,
            push_magic: None,
            push_token: None,
            disabled: true,
            authenticate_raw: Some(&msg.raw),
            token_update_raw: None,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(enrollments::table)
            .values(&new_enrollment)
            .on_conflict(enrollments::id)
            .do_update()
            .set((
                enrollments::topic.eq(&msg.topic),
                enrollments::disabled.eq(true),
                enrollments::authenticate_raw.eq(Some(&msg.raw)),
                enrollments::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .wrap_err("failed to store authenticate")?;

        Ok(())
    }

    fn store_token_update(
        &self,
        id: &EnrollId,
        msg: &mdm_core::TokenUpdate,
    ) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;
        let now = chrono::Utc::now().naive_utc();

        diesel::update(enrollments::table.filter(enrollments::id.eq(&id.id)))
            .set((
                enrollments::push_magic.eq(Some(&msg.push_magic)),
                enrollments::push_token.eq(Some(&msg.token)),
                enrollments::disabled.eq(false),
                enrollments::token_update_raw.eq(Some(&msg.raw)),
                enrollments::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .wrap_err("failed to store token update")?;

        Ok(())
    }

    fn store_checkout(
        &self,
        id: &EnrollId,
        _msg: &mdm_core::CheckOut,
    ) -> color_eyre::eyre::Result<()> {
        self.disable(id)
    }

    fn is_disabled(&self, id: &EnrollId) -> color_eyre::eyre::Result<bool> {
        let mut conn = self.conn()?;

        let result: Option<bool> = enrollments::table
            .filter(enrollments::id.eq(&id.id))
            .select(enrollments::disabled)
            .first(&mut conn)
            .optional()
            .wrap_err("failed to check disabled status")?;

        Ok(result.unwrap_or(true))
    }

    fn disable(&self, id: &EnrollId) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        diesel::update(enrollments::table.filter(enrollments::id.eq(&id.id)))
            .set(enrollments::disabled.eq(true))
            .execute(&mut conn)
            .wrap_err("failed to disable enrollment")?;

        Ok(())
    }
}

impl CommandStore for SqliteStorage {
    fn enqueue_command(&self, id: &EnrollId, command: &[u8]) -> color_eyre::eyre::Result<String> {
        let mut conn = self.conn()?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();

        let new_command = NewCommand {
            enrollment_id: &id.id,
            uuid: &uuid,
            command,
            status: "Pending",
            created_at: now,
        };

        diesel::insert_into(commands::table)
            .values(&new_command)
            .execute(&mut conn)
            .wrap_err("failed to enqueue command")?;

        Ok(uuid)
    }

    fn next_command(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<QueuedCommand>> {
        let mut conn = self.conn()?;

        let result: Option<CommandRow> = commands::table
            .filter(commands::enrollment_id.eq(&id.id))
            .filter(commands::status.eq("Pending"))
            .order(commands::created_at.asc())
            .first(&mut conn)
            .optional()
            .wrap_err("failed to get next command")?;

        Ok(result.map(|row| QueuedCommand {
            uuid: row.uuid,
            command: row.command,
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
        }))
    }

    fn store_result(
        &self,
        id: &EnrollId,
        results: &mdm_core::CommandResults,
    ) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        diesel::update(
            commands::table
                .filter(commands::enrollment_id.eq(&id.id))
                .filter(commands::uuid.eq(&results.command_uuid)),
        )
        .set((
            commands::status.eq(results.status.to_string()),
            commands::result.eq(Some(&results.raw)),
        ))
        .execute(&mut conn)
        .wrap_err("failed to store command result")?;

        Ok(())
    }

    fn clear_queue(&self, id: &EnrollId) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        diesel::delete(commands::table.filter(commands::enrollment_id.eq(&id.id)))
            .execute(&mut conn)
            .wrap_err("failed to clear command queue")?;

        Ok(())
    }
}

impl BootstrapTokenStore for SqliteStorage {
    fn store_bootstrap_token(&self, id: &EnrollId, token: &[u8]) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        let new_token = NewBootstrapToken {
            enrollment_id: &id.id,
            token,
        };

        diesel::insert_into(bootstrap_tokens::table)
            .values(&new_token)
            .on_conflict(bootstrap_tokens::enrollment_id)
            .do_update()
            .set(bootstrap_tokens::token.eq(token))
            .execute(&mut conn)
            .wrap_err("failed to store bootstrap token")?;

        Ok(())
    }

    fn get_bootstrap_token(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        let mut conn = self.conn()?;

        let result: Option<Vec<u8>> = bootstrap_tokens::table
            .filter(bootstrap_tokens::enrollment_id.eq(&id.id))
            .select(bootstrap_tokens::token)
            .first(&mut conn)
            .optional()
            .wrap_err("failed to get bootstrap token")?;

        Ok(result)
    }

    fn delete_bootstrap_token(&self, id: &EnrollId) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        diesel::delete(bootstrap_tokens::table.filter(bootstrap_tokens::enrollment_id.eq(&id.id)))
            .execute(&mut conn)
            .wrap_err("failed to delete bootstrap token")?;

        Ok(())
    }
}

impl PushStore for SqliteStorage {
    fn get_push_info(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<PushInfo>> {
        let mut conn = self.conn()?;

        let result: Option<(Option<String>, Option<Vec<u8>>, String)> = enrollments::table
            .filter(enrollments::id.eq(&id.id))
            .filter(enrollments::disabled.eq(false))
            .select((
                enrollments::push_magic,
                enrollments::push_token,
                enrollments::topic,
            ))
            .first(&mut conn)
            .optional()
            .wrap_err("failed to get push info")?;

        Ok(
            result.and_then(|(magic, token, topic)| match (magic, token) {
                (Some(push_magic), Some(token)) => Some(PushInfo {
                    token,
                    push_magic,
                    topic,
                }),
                _ => None,
            }),
        )
    }

    fn get_push_infos(&self, ids: &[&EnrollId]) -> color_eyre::eyre::Result<Vec<PushInfo>> {
        let id_strings: Vec<&str> = ids.iter().map(|id| id.id.as_str()).collect();
        let mut conn = self.conn()?;

        let results: Vec<(String, Option<String>, Option<Vec<u8>>, String)> = enrollments::table
            .filter(enrollments::id.eq_any(&id_strings))
            .filter(enrollments::disabled.eq(false))
            .select((
                enrollments::id,
                enrollments::push_magic,
                enrollments::push_token,
                enrollments::topic,
            ))
            .load(&mut conn)
            .wrap_err("failed to get push infos")?;

        Ok(results
            .into_iter()
            .filter_map(|(_, magic, token, topic)| match (magic, token) {
                (Some(push_magic), Some(token)) => Some(PushInfo {
                    token,
                    push_magic,
                    topic,
                }),
                _ => None,
            })
            .collect())
    }
}

impl PushCertStore for SqliteStorage {
    fn store_push_cert(
        &self,
        topic: &str,
        cert_pem: &str,
        key_pem: &str,
    ) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        let new_cert = NewPushCert {
            topic,
            cert_pem,
            key_pem,
            not_after: None, // TODO: Parse from cert
        };

        diesel::insert_into(push_certs::table)
            .values(&new_cert)
            .on_conflict(push_certs::topic)
            .do_update()
            .set((
                push_certs::cert_pem.eq(cert_pem),
                push_certs::key_pem.eq(key_pem),
            ))
            .execute(&mut conn)
            .wrap_err("failed to store push cert")?;

        Ok(())
    }

    fn get_push_cert(&self, topic: &str) -> color_eyre::eyre::Result<Option<(String, String)>> {
        let mut conn = self.conn()?;

        let result: Option<(String, String)> = push_certs::table
            .filter(push_certs::topic.eq(topic))
            .select((push_certs::cert_pem, push_certs::key_pem))
            .first(&mut conn)
            .optional()
            .wrap_err("failed to get push cert")?;

        Ok(result)
    }
}

impl CertAuthStore for SqliteStorage {
    fn associate_cert(&self, id: &EnrollId, cert_hash: &[u8]) -> color_eyre::eyre::Result<()> {
        let mut conn = self.conn()?;

        let new_auth = NewCertAuth {
            enrollment_id: &id.id,
            cert_hash,
        };

        diesel::insert_into(cert_auth::table)
            .values(&new_auth)
            .execute(&mut conn)
            .wrap_err("failed to associate cert")?;

        Ok(())
    }

    fn has_cert_auth(&self, id: &EnrollId, cert_hash: &[u8]) -> color_eyre::eyre::Result<bool> {
        let mut conn = self.conn()?;

        let count: i64 = cert_auth::table
            .filter(cert_auth::enrollment_id.eq(&id.id))
            .filter(cert_auth::cert_hash.eq(cert_hash))
            .count()
            .get_result(&mut conn)
            .wrap_err("failed to check cert auth")?;

        Ok(count > 0)
    }
}
