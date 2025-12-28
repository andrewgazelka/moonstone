//! Diesel schema definitions.

diesel::table! {
    enrollments (id) {
        id -> Text,
        enroll_type -> Text,
        device_id -> Nullable<Text>,
        parent_id -> Nullable<Text>,
        topic -> Text,
        push_magic -> Nullable<Text>,
        push_token -> Nullable<Binary>,
        disabled -> Bool,
        authenticate_raw -> Nullable<Binary>,
        token_update_raw -> Nullable<Binary>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    commands (id) {
        id -> Integer,
        enrollment_id -> Text,
        uuid -> Text,
        command -> Binary,
        status -> Text,
        result -> Nullable<Binary>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    push_certs (topic) {
        topic -> Text,
        cert_pem -> Text,
        key_pem -> Text,
        not_after -> Nullable<Timestamp>,
    }
}

diesel::table! {
    bootstrap_tokens (enrollment_id) {
        enrollment_id -> Text,
        token -> Binary,
    }
}

diesel::table! {
    cert_auth (id) {
        id -> Integer,
        enrollment_id -> Text,
        cert_hash -> Binary,
    }
}

diesel::joinable!(commands -> enrollments (enrollment_id));
diesel::joinable!(bootstrap_tokens -> enrollments (enrollment_id));

diesel::allow_tables_to_appear_in_same_query!(
    enrollments,
    commands,
    push_certs,
    bootstrap_tokens,
    cert_auth,
);
