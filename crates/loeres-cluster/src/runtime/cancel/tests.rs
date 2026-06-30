use super::*;

#[test]
fn new_token_is_not_cancelled() {
    assert!(!ClusterCancellationToken::new().is_cancelled());
}

#[test]
fn cancel_is_observed() {
    let token = ClusterCancellationToken::new();
    token.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn clones_share_one_flag() {
    let token = ClusterCancellationToken::new();
    let clone = token.clone();
    token.cancel();
    assert!(clone.is_cancelled());
}

#[test]
fn cancel_is_idempotent() {
    let token = ClusterCancellationToken::new();
    token.cancel();
    token.cancel();
    assert!(token.is_cancelled());
}
