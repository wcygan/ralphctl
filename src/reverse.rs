//! Reverse mode implementation for ralphctl.
//!
//! Provides investigation loop logic distinct from forward mode.
//! Reverse mode is used for autonomous investigation of codebases
//! to answer questions—diagnosing bugs, understanding systems, or
//! mapping dependencies before changes.

#![allow(dead_code)] // Components used by future reverse mode implementation

/// Reverse mode signal types.
///
/// These signals control the reverse mode investigation loop.
/// Detection priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseSignal {
    /// Still investigating, more hypotheses to explore
    Continue,
    /// Question answered, FINDINGS.md written
    Found(String),
    /// Cannot determine answer, FINDINGS.md written with what was tried
    Inconclusive(String),
    /// Cannot proceed, requires human intervention
    Blocked(String),
    /// No signal detected in output
    NoSignal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_signal_equality() {
        assert_eq!(ReverseSignal::Continue, ReverseSignal::Continue);
        assert_eq!(ReverseSignal::NoSignal, ReverseSignal::NoSignal);
        assert_eq!(
            ReverseSignal::Found("answer".to_string()),
            ReverseSignal::Found("answer".to_string())
        );
        assert_eq!(
            ReverseSignal::Inconclusive("reason".to_string()),
            ReverseSignal::Inconclusive("reason".to_string())
        );
        assert_eq!(
            ReverseSignal::Blocked("blocker".to_string()),
            ReverseSignal::Blocked("blocker".to_string())
        );
    }

    #[test]
    fn test_reverse_signal_inequality() {
        assert_ne!(ReverseSignal::Continue, ReverseSignal::NoSignal);
        assert_ne!(
            ReverseSignal::Found("a".to_string()),
            ReverseSignal::Found("b".to_string())
        );
        assert_ne!(
            ReverseSignal::Found("x".to_string()),
            ReverseSignal::Inconclusive("x".to_string())
        );
    }

    #[test]
    fn test_reverse_signal_clone() {
        let signal = ReverseSignal::Found("discovery".to_string());
        let cloned = signal.clone();
        assert_eq!(signal, cloned);

        let signal2 = ReverseSignal::Continue;
        let cloned2 = signal2.clone();
        assert_eq!(signal2, cloned2);
    }

    #[test]
    fn test_reverse_signal_debug() {
        let signal = ReverseSignal::Found("test".to_string());
        let debug_str = format!("{:?}", signal);
        assert!(debug_str.contains("Found"));
        assert!(debug_str.contains("test"));

        let signal2 = ReverseSignal::Continue;
        let debug_str2 = format!("{:?}", signal2);
        assert_eq!(debug_str2, "Continue");

        let signal3 = ReverseSignal::NoSignal;
        let debug_str3 = format!("{:?}", signal3);
        assert_eq!(debug_str3, "NoSignal");
    }

    #[test]
    fn test_reverse_signal_blocked_with_reason() {
        let reason = "missing credentials".to_string();
        let signal = ReverseSignal::Blocked(reason.clone());
        if let ReverseSignal::Blocked(r) = signal {
            assert_eq!(r, reason);
        } else {
            panic!("Expected Blocked variant");
        }
    }

    #[test]
    fn test_reverse_signal_inconclusive_with_reason() {
        let reason = "not enough evidence".to_string();
        let signal = ReverseSignal::Inconclusive(reason.clone());
        if let ReverseSignal::Inconclusive(r) = signal {
            assert_eq!(r, reason);
        } else {
            panic!("Expected Inconclusive variant");
        }
    }
}
