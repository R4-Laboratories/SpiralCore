//! Local timestamp construction policy. Canonical receiving rules stay strict.
use crate::service::{ServiceError, ServiceTime};
type Result<T> = std::result::Result<T, ServiceError>;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IssuancePolicy {
    #[default]
    Exact,
    /// Backdate once to tolerate a sender clock ahead by this bounded amount.
    /// The backdate reduces remaining lifetime; it never extends the 30s TTL.
    Bounded { max_clock_ahead_ms: u64 },
}
pub(crate) struct IssuedWindow {
    pub issued: i64,
    pub expires: i64,
}
impl IssuancePolicy {
    pub fn bounded(max_clock_ahead_ms: u64) -> Result<Self> {
        let policy = Self::Bounded { max_clock_ahead_ms };
        policy.validate()?;
        Ok(policy)
    }
    pub(crate) fn validate(self) -> Result<()> {
        if matches!(self,Self::Bounded{max_clock_ahead_ms} if !(1..=5000).contains(&max_clock_ahead_ms))
        {
            return Err(ServiceError::Limit);
        }
        Ok(())
    }
    pub(crate) fn window(
        self,
        now: ServiceTime,
        requested_expiry: i64,
        not_before: i64,
        valid_until: i64,
    ) -> Result<IssuedWindow> {
        self.validate()?;
        let lag = match self {
            Self::Exact => 0,
            Self::Bounded { max_clock_ahead_ms } => max_clock_ahead_ms as i64,
        };
        let issued = now.unix_ms.checked_sub(lag).ok_or(ServiceError::Clock)?;
        // Never clamp upward: that would silently reduce the declared clock
        // bound. A grant must already cover the complete issuance interval.
        if issued < not_before {
            return Err(ServiceError::Unauthorized);
        }
        let expires = requested_expiry
            .min(issued.checked_add(30_000).ok_or(ServiceError::Clock)?)
            .min(valid_until);
        if expires <= now.unix_ms {
            return Err(ServiceError::Expired);
        }
        Ok(IssuedWindow { issued, expires })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> ServiceTime {
        ServiceTime {
            unix_ms: 10_000,
            monotonic_ms: 100,
        }
    }
    #[test]
    fn bounded_issuance_shortens_remaining_time_and_never_clamps_a_young_grant() {
        let policy = IssuancePolicy::bounded(5000).unwrap();
        let w = policy.window(now(), 40_000, 0, 100_000).unwrap();
        assert_eq!((w.issued, w.expires), (5000, 35000));
        assert!(matches!(
            policy.window(now(), 40_000, 5001, 100_000),
            Err(ServiceError::Unauthorized)
        ));
        let w = policy.window(now(), 20_000, 0, 19_000).unwrap();
        assert_eq!((w.issued, w.expires), (5000, 19000));
        assert!(matches!(
            policy.window(now(), 10_000, 0, 100_000),
            Err(ServiceError::Expired)
        ));
    }
    #[test]
    fn policy_bounds_and_arithmetic_are_checked() {
        assert!(IssuancePolicy::bounded(5001).is_err());
        assert!(IssuancePolicy::bounded(0).is_err());
        assert!(matches!(
            IssuancePolicy::Bounded {
                max_clock_ahead_ms: 5001
            }
            .window(now(), 20_000, 0, 30_000),
            Err(ServiceError::Limit)
        ));
        assert!(matches!(
            IssuancePolicy::bounded(5000).unwrap().window(
                ServiceTime {
                    unix_ms: i64::MIN,
                    monotonic_ms: 0
                },
                0,
                i64::MIN,
                100
            ),
            Err(ServiceError::Clock)
        ));
        assert!(matches!(
            IssuancePolicy::Exact.window(
                ServiceTime {
                    unix_ms: i64::MAX - 1,
                    monotonic_ms: 0
                },
                i64::MAX,
                0,
                i64::MAX
            ),
            Err(ServiceError::Clock)
        ));
    }
}
