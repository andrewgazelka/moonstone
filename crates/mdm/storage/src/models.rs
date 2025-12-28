//! Database models.

use diesel::prelude::*;

use crate::schema::{bootstrap_tokens, cert_auth, commands, enrollments, push_certs};

/// Enrollment record.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = enrollments)]
pub struct EnrollmentRow {
    pub id: String,
    pub enroll_type: String,
    pub device_id: Option<String>,
    pub parent_id: Option<String>,
    pub topic: String,
    pub push_magic: Option<String>,
    pub push_token: Option<Vec<u8>>,
    pub disabled: bool,
    pub authenticate_raw: Option<Vec<u8>>,
    pub token_update_raw: Option<Vec<u8>>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// New enrollment for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = enrollments)]
pub struct NewEnrollment<'a> {
    pub id: &'a str,
    pub enroll_type: &'a str,
    pub device_id: Option<&'a str>,
    pub parent_id: Option<&'a str>,
    pub topic: &'a str,
    pub push_magic: Option<&'a str>,
    pub push_token: Option<&'a [u8]>,
    pub disabled: bool,
    pub authenticate_raw: Option<&'a [u8]>,
    pub token_update_raw: Option<&'a [u8]>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Command record.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = commands)]
pub struct CommandRow {
    pub id: i32,
    pub enrollment_id: String,
    pub uuid: String,
    pub command: Vec<u8>,
    pub status: String,
    pub result: Option<Vec<u8>>,
    pub created_at: chrono::NaiveDateTime,
}

/// New command for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = commands)]
pub struct NewCommand<'a> {
    pub enrollment_id: &'a str,
    pub uuid: &'a str,
    pub command: &'a [u8],
    pub status: &'a str,
    pub created_at: chrono::NaiveDateTime,
}

/// Push certificate record.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = push_certs, primary_key(topic))]
pub struct PushCertRow {
    pub topic: String,
    pub cert_pem: String,
    pub key_pem: String,
    pub not_after: Option<chrono::NaiveDateTime>,
}

/// New push certificate for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = push_certs)]
pub struct NewPushCert<'a> {
    pub topic: &'a str,
    pub cert_pem: &'a str,
    pub key_pem: &'a str,
    pub not_after: Option<chrono::NaiveDateTime>,
}

/// Bootstrap token record.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = bootstrap_tokens, primary_key(enrollment_id))]
pub struct BootstrapTokenRow {
    pub enrollment_id: String,
    pub token: Vec<u8>,
}

/// New bootstrap token for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = bootstrap_tokens)]
pub struct NewBootstrapToken<'a> {
    pub enrollment_id: &'a str,
    pub token: &'a [u8],
}

/// Certificate auth record.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = cert_auth)]
pub struct CertAuthRow {
    pub id: i32,
    pub enrollment_id: String,
    pub cert_hash: Vec<u8>,
}

/// New certificate auth for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = cert_auth)]
pub struct NewCertAuth<'a> {
    pub enrollment_id: &'a str,
    pub cert_hash: &'a [u8],
}
