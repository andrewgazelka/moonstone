//! Enrollment types for device and user identification.

/// Type of MDM enrollment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EnrollType {
    /// Standard device enrollment (UDID-based).
    Device,
    /// User channel on a device.
    User,
    /// User Enrollment device (modern DEP-like).
    UserEnrollmentDevice,
    /// User channel on UserEnrollmentDevice.
    UserEnrollment,
    /// Shared iPad with static UserID.
    SharedIpad,
}

/// Raw enrollment data from check-in messages.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Enrollment {
    /// Device UDID (legacy enrollments).
    #[serde(default, rename = "UDID")]
    pub udid: Option<String>,

    /// User ID for user-channel enrollments.
    #[serde(default, rename = "UserID")]
    pub user_id: Option<String>,

    /// User short name (Managed Apple ID).
    #[serde(default)]
    pub user_short_name: Option<String>,

    /// User long name.
    #[serde(default)]
    pub user_long_name: Option<String>,

    /// Enrollment ID for User Enrollment devices.
    #[serde(default, rename = "EnrollmentID")]
    pub enrollment_id: Option<String>,

    /// Enrollment User ID for User Enrollment.
    #[serde(default, rename = "EnrollmentUserID")]
    pub enrollment_user_id: Option<String>,
}

/// Resolved enrollment identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EnrollId {
    /// The enrollment type.
    pub enroll_type: EnrollType,
    /// Primary identifier (device ID).
    pub id: String,
    /// Parent device ID (for user channels).
    pub parent_id: Option<String>,
}

impl Enrollment {
    /// Resolve this enrollment to an EnrollId.
    ///
    /// Returns the resolved ID and enrollment type based on available fields.
    pub fn resolve(&self) -> Option<EnrollId> {
        // User Enrollment (modern)
        if let Some(ref enrollment_id) = self.enrollment_id {
            if let Some(ref enrollment_user_id) = self.enrollment_user_id {
                return Some(EnrollId {
                    enroll_type: EnrollType::UserEnrollment,
                    id: format!("{}:{}", enrollment_id, enrollment_user_id),
                    parent_id: Some(enrollment_id.clone()),
                });
            }
            return Some(EnrollId {
                enroll_type: EnrollType::UserEnrollmentDevice,
                id: enrollment_id.clone(),
                parent_id: None,
            });
        }

        // Legacy UDID-based enrollment
        if let Some(ref udid) = self.udid {
            if let Some(ref user_id) = self.user_id {
                // Shared iPad uses static "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"
                let enroll_type = if user_id == "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF" {
                    EnrollType::SharedIpad
                } else {
                    EnrollType::User
                };
                return Some(EnrollId {
                    enroll_type,
                    id: format!("{}:{}", udid, user_id),
                    parent_id: Some(udid.clone()),
                });
            }
            return Some(EnrollId {
                enroll_type: EnrollType::Device,
                id: udid.clone(),
                parent_id: None,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_enrollment() {
        let enrollment = Enrollment {
            udid: Some("ABC123".into()),
            ..Default::default()
        };
        let id = enrollment.resolve().unwrap();
        assert_eq!(id.enroll_type, EnrollType::Device);
        assert_eq!(id.id, "ABC123");
        assert!(id.parent_id.is_none());
    }

    #[test]
    fn test_user_enrollment() {
        let enrollment = Enrollment {
            udid: Some("ABC123".into()),
            user_id: Some("user-456".into()),
            ..Default::default()
        };
        let id = enrollment.resolve().unwrap();
        assert_eq!(id.enroll_type, EnrollType::User);
        assert_eq!(id.id, "ABC123:user-456");
        assert_eq!(id.parent_id.as_deref(), Some("ABC123"));
    }
}
