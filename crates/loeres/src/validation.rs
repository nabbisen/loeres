//! Validation state and trusted-input vocabulary (RFC 012).
//!
//! Core, allocation-free vocabulary describing *what input validation has been
//! performed*. RFC 012 owns only the representation: it runs no scans and
//! changes no solver signature. Backends remain the actual validators
//! (`loeres-backend-std::*::validate_finite`, the `loeres-device` inline
//! pre-iteration checks) and record their outcome here.
//!
//! Structural validity — dimensions, sparse coordinate bounds, duplicate
//! rejection — is a construction precondition owned by the storage constructors
//! (RFC 004 / RFC 007) and is **not** represented here. These types cover the
//! remaining runtime / semantic checks.

/// A compact set of validation coverage dimensions.
///
/// A `repr(transparent)` newtype over `u8`, one bit per dimension. Structural
/// dimensions / bounds are construction-owned and are not bits here.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ValidationScope(u8);

impl ValidationScope {
    /// The empty scope (no dimensions covered).
    pub const EMPTY: Self = Self(0);
    /// Finite-value coverage.
    pub const FINITE: Self = Self(0b0000_0001);
    /// Problem / config pairing coverage.
    pub const PROBLEM_CONFIG: Self = Self(0b0000_0010);
    /// Solver-family pre-loop invariant coverage.
    pub const PRELOOP: Self = Self(0b0000_0100);
    /// All validation dimensions **known to this release** — composed from the
    /// current bits, not a forever-complete claim. Later RFCs that add a
    /// dimension redefine `ALL` for their release.
    pub const ALL: Self = Self(Self::FINITE.0 | Self::PROBLEM_CONFIG.0 | Self::PRELOOP.0);

    /// The empty scope.
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Whether `self` contains every bit in `other`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// The union of two scopes.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The intersection of two scopes.
    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl core::ops::BitOr for ValidationScope {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl core::ops::BitAnd for ValidationScope {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        self.intersect(rhs)
    }
}

/// Whether a finite-value scan was performed or was not applicable.
///
/// `NotApplicable` is permitted only when the scalar/domain is explicitly
/// non-finite-incapable by its type/domain contract — never merely because a
/// `FiniteScalar` impl is absent (that is an unavailable capability, which must
/// be rejected rather than recorded as `NotApplicable`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FiniteCoverage {
    /// A finite scan ran (`S: FiniteScalar`) and passed.
    Checked,
    /// Non-finite values are impossible by the scalar's domain / type contract.
    NotApplicable,
}

/// The kind of trusted responsibility transfer.
///
/// `#[non_exhaustive]` so later RFCs (e.g. RFC 008 pipeline trust) can add
/// categories without a breaking change.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TrustKind {
    /// The caller explicitly assumed responsibility for the asserted scope.
    CallerAssertion,
}

/// A compact numeric audit token for a trusted assertion.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TrustToken(u32);

impl TrustToken {
    /// Wrap an audit token value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The token value.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// What invariant coverage a [`ValidationState::Validated`] recorded.
///
/// A *recording* descriptor, not a proof: construct it only after the owning
/// backend / solver has actually run the relevant checks, or when an invariant
/// is explicitly not applicable.
///
/// Coherent by construction: every `ValidationCoverage` addresses finite
/// coverage (via [`finite`](ValidationCoverage::finite)), so [`new`] normalizes
/// the scope to always include [`ValidationScope::FINITE`]; the scope bit and
/// the `finite` field can never contradict. Fields are private so the invariant
/// cannot be bypassed by a struct literal — read via the accessors.
///
/// [`new`]: ValidationCoverage::new
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ValidationCoverage {
    scope: ValidationScope,
    finite: FiniteCoverage,
}

impl ValidationCoverage {
    /// Record coverage. Callers construct this only after the relevant checks
    /// have actually run, or when the invariant is explicitly not applicable.
    ///
    /// `scope` is normalized to include [`ValidationScope::FINITE`] (finite is
    /// always addressed — by `finite`); the remaining scope bits record whether
    /// the other invariants were addressed.
    pub const fn new(scope: ValidationScope, finite: FiniteCoverage) -> Self {
        Self {
            scope: scope.union(ValidationScope::FINITE),
            finite,
        }
    }

    /// The coverage scope (always includes [`ValidationScope::FINITE`]).
    pub const fn scope(self) -> ValidationScope {
        self.scope
    }

    /// How the finite invariant was addressed.
    pub const fn finite(self) -> FiniteCoverage {
        self.finite
    }
}

/// Evidence that the caller assumed responsibility for a coverage scope.
///
/// Responsibility transfer is visible in the asserted `scope`; it is not a
/// correctness proof.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TrustedByCaller {
    /// The coverage scope the caller asserts responsibility for.
    pub scope: ValidationScope,
    /// The kind of trust.
    pub kind: TrustKind,
    /// A compact audit token.
    pub token: TrustToken,
    /// An optional static label.
    pub label: Option<&'static str>,
}

impl TrustedByCaller {
    /// A caller-assertion trust over `scope`, with `token` and optional `label`.
    pub const fn caller_assertion(
        scope: ValidationScope,
        token: TrustToken,
        label: Option<&'static str>,
    ) -> Self {
        Self {
            scope,
            kind: TrustKind::CallerAssertion,
            token,
            label,
        }
    }
}

/// The validation state of an input at a solve boundary.
///
/// Structural validity is a construction precondition (RFC 004 / RFC 007) and
/// is not a state here; these categories cover the remaining runtime / semantic
/// checks and responsibility transfer.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidationState {
    /// The remaining runtime / semantic checks have not been performed.
    Unvalidated,
    /// Loeres checked the applicable remaining invariants.
    Validated(ValidationCoverage),
    /// The caller assumed responsibility for a coverage scope.
    Trusted(TrustedByCaller),
}

#[cfg(test)]
mod tests;
