use thiserror::Error;

/// SDP API error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdpErrorCode {
    Success = 2000,
    InvalidValue = 4001,
    Forbidden = 4002,
    ClosureRuleViolation = 4003,
    Internal = 4004,
    ReferenceExists = 4005,
    NotFound = 4007,
    NotUnique = 4008,
    NonEditableField = 4009,
    InternalField = 4010,
    NoSuchField = 4011,
    MissingMandatoryField = 4012,
    UnsupportedContentType = 4013,
    ReadOnlyField = 4014,
    RateLimitExceeded = 4015,
    AlreadyInTrash = 4016,
    NotInTrash = 4017,
    LicenseRestriction = 7001,
    Unknown = 0,
}

impl From<u32> for SdpErrorCode {
    fn from(code: u32) -> Self {
        match code {
            2000 => SdpErrorCode::Success,
            4001 => SdpErrorCode::InvalidValue,
            4002 => SdpErrorCode::Forbidden,
            4003 => SdpErrorCode::ClosureRuleViolation,
            4004 => SdpErrorCode::Internal,
            4005 => SdpErrorCode::ReferenceExists,
            4007 => SdpErrorCode::NotFound,
            4008 => SdpErrorCode::NotUnique,
            4009 => SdpErrorCode::NonEditableField,
            4010 => SdpErrorCode::InternalField,
            4011 => SdpErrorCode::NoSuchField,
            4012 => SdpErrorCode::MissingMandatoryField,
            4013 => SdpErrorCode::UnsupportedContentType,
            4014 => SdpErrorCode::ReadOnlyField,
            4015 => SdpErrorCode::RateLimitExceeded,
            4016 => SdpErrorCode::AlreadyInTrash,
            4017 => SdpErrorCode::NotInTrash,
            7001 => SdpErrorCode::LicenseRestriction,
            _ => SdpErrorCode::Unknown,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Authentication failed: invalid or expired token")]
    Unauthorized,
    #[error("Permission denied: {0}")]
    Forbidden(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
    #[error("Resource already exists (not unique): {0}")]
    NotUnique(String),
    #[error("Cannot delete: resource is referenced elsewhere")]
    ReferenceExists,
    #[error("Missing mandatory field: {0}")]
    MissingField(String),
    #[error("Field is not editable: {0}")]
    NotEditable(String),
    #[error("Field does not exist: {0}")]
    NoSuchField(String),
    #[error("Closure rule violation: {0}")]
    ClosureRuleViolation(String),
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("License restriction: operation not allowed")]
    LicenseRestricted,
    #[error("SDP internal error")]
    Internal,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Form encoding error: {0}")]
    FormEncoding(#[from] serde_urlencoded::ser::Error),
    #[error("SDP error (code {code}): {message}")]
    Sdp { code: u32, message: String },
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an error from SDP response status code and message
    pub fn from_sdp(code: u32, message: String, field: Option<String>) -> Self {
        let field_info = field.clone().unwrap_or_else(|| message.clone());

        match SdpErrorCode::from(code) {
            SdpErrorCode::InvalidValue => Error::InvalidValue(field_info),
            SdpErrorCode::Forbidden => Error::Forbidden(field_info),
            SdpErrorCode::ClosureRuleViolation => Error::ClosureRuleViolation(field_info),
            SdpErrorCode::Internal => Error::Internal,
            SdpErrorCode::ReferenceExists => Error::ReferenceExists,
            SdpErrorCode::NotFound => Error::NotFound(field_info),
            SdpErrorCode::NotUnique => Error::NotUnique(field_info),
            SdpErrorCode::NonEditableField | SdpErrorCode::ReadOnlyField => {
                Error::NotEditable(field_info)
            }
            SdpErrorCode::InternalField => {
                Error::NotEditable(format!("internal field: {}", field_info))
            }
            SdpErrorCode::NoSuchField => Error::NoSuchField(field_info),
            SdpErrorCode::MissingMandatoryField => Error::MissingField(field_info),
            SdpErrorCode::RateLimitExceeded => Error::RateLimited,
            SdpErrorCode::LicenseRestriction => Error::LicenseRestricted,
            SdpErrorCode::AlreadyInTrash | SdpErrorCode::NotInTrash => Error::Sdp { code, message },
            SdpErrorCode::UnsupportedContentType => Error::Sdp { code, message },
            SdpErrorCode::Success => Error::Other("Unexpected success code in error path".into()),
            SdpErrorCode::Unknown => Error::Sdp { code, message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdp_error_code_from_u32() {
        assert_eq!(SdpErrorCode::from(2000), SdpErrorCode::Success);
        assert_eq!(SdpErrorCode::from(4001), SdpErrorCode::InvalidValue);
        assert_eq!(SdpErrorCode::from(4002), SdpErrorCode::Forbidden);
        assert_eq!(SdpErrorCode::from(4003), SdpErrorCode::ClosureRuleViolation);
        assert_eq!(SdpErrorCode::from(4004), SdpErrorCode::Internal);
        assert_eq!(SdpErrorCode::from(4005), SdpErrorCode::ReferenceExists);
        assert_eq!(SdpErrorCode::from(4007), SdpErrorCode::NotFound);
        assert_eq!(SdpErrorCode::from(4008), SdpErrorCode::NotUnique);
        assert_eq!(SdpErrorCode::from(4009), SdpErrorCode::NonEditableField);
        assert_eq!(SdpErrorCode::from(4010), SdpErrorCode::InternalField);
        assert_eq!(SdpErrorCode::from(4011), SdpErrorCode::NoSuchField);
        assert_eq!(
            SdpErrorCode::from(4012),
            SdpErrorCode::MissingMandatoryField
        );
        assert_eq!(
            SdpErrorCode::from(4013),
            SdpErrorCode::UnsupportedContentType
        );
        assert_eq!(SdpErrorCode::from(4014), SdpErrorCode::ReadOnlyField);
        assert_eq!(SdpErrorCode::from(4015), SdpErrorCode::RateLimitExceeded);
        assert_eq!(SdpErrorCode::from(4016), SdpErrorCode::AlreadyInTrash);
        assert_eq!(SdpErrorCode::from(4017), SdpErrorCode::NotInTrash);
        assert_eq!(SdpErrorCode::from(7001), SdpErrorCode::LicenseRestriction);
        assert_eq!(SdpErrorCode::from(9999), SdpErrorCode::Unknown);
        assert_eq!(SdpErrorCode::from(0), SdpErrorCode::Unknown);
    }

    #[test]
    fn error_from_sdp_maps_correctly() {
        assert!(matches!(
            Error::from_sdp(4001, "msg".into(), None),
            Error::InvalidValue(_)
        ));
        assert!(matches!(
            Error::from_sdp(4002, "msg".into(), None),
            Error::Forbidden(_)
        ));
        assert!(matches!(
            Error::from_sdp(4004, "msg".into(), None),
            Error::Internal
        ));
        assert!(matches!(
            Error::from_sdp(4005, "msg".into(), None),
            Error::ReferenceExists
        ));
        assert!(matches!(
            Error::from_sdp(4007, "msg".into(), None),
            Error::NotFound(_)
        ));
        assert!(matches!(
            Error::from_sdp(4009, "msg".into(), None),
            Error::NotEditable(_)
        ));
        assert!(matches!(
            Error::from_sdp(4014, "msg".into(), None),
            Error::NotEditable(_)
        ));
        assert!(matches!(
            Error::from_sdp(4012, "msg".into(), None),
            Error::MissingField(_)
        ));
        assert!(matches!(
            Error::from_sdp(4015, "msg".into(), None),
            Error::RateLimited
        ));
        assert!(matches!(
            Error::from_sdp(7001, "msg".into(), None),
            Error::LicenseRestricted
        ));
        assert!(matches!(
            Error::from_sdp(9999, "msg".into(), None),
            Error::Sdp { .. }
        ));
    }

    #[test]
    fn error_from_sdp_uses_field_when_provided() {
        let err = Error::from_sdp(4001, "message".into(), Some("field_name".into()));
        match err {
            Error::InvalidValue(s) => assert_eq!(s, "field_name"),
            _ => panic!("expected InvalidValue"),
        }
    }

    #[test]
    fn error_from_sdp_uses_message_when_no_field() {
        let err = Error::from_sdp(4001, "message".into(), None);
        match err {
            Error::InvalidValue(s) => assert_eq!(s, "message"),
            _ => panic!("expected InvalidValue"),
        }
    }
}
