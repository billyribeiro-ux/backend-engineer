use uuid::Uuid;

use crate::{auth::extractor::AuthUser, models::subscription::Subscription};

/// Can the user edit the given profile?
pub fn can_edit_profile(actor: &AuthUser, target_user_id: Uuid, target_is_org_owner: bool) -> bool {
    match actor.role.as_str() {
        "admin" => {
            // Admins can edit anyone except the org owner (unless admin IS the org owner)
            if target_is_org_owner && actor.id != target_user_id {
                return false;
            }
            true
        }
        _ => actor.id == target_user_id,
    }
}

/// Can the user view the given subscription?
pub fn can_view_subscription(actor: &AuthUser, subscription: &Subscription) -> bool {
    matches!(actor.role.as_str(), "admin" | "support")
        || subscription.user_id == actor.id
}

/// Can the user cancel the given subscription?
pub fn can_cancel_subscription(actor: &AuthUser, subscription: &Subscription) -> bool {
    if subscription.is_canceled() {
        return false;
    }
    if actor.role == "admin" {
        return true;
    }
    subscription.user_id == actor.id
}

/// Does the user have an active subscription (active or trialing)?
pub fn has_active_subscription(subscription: Option<&Subscription>) -> bool {
    subscription
        .map(|s| s.has_active_access())
        .unwrap_or(false)
}

/// Can the user delete the given user account?
pub fn can_delete_user(actor: &AuthUser, target_user_id: Uuid, target_is_org_owner: bool) -> bool {
    if actor.role != "admin" {
        return false;
    }
    if actor.id == target_user_id {
        return false; // Can't delete yourself
    }
    if target_is_org_owner {
        return false; // Can't delete org owner
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(id: Uuid, role: &str) -> AuthUser {
        AuthUser {
            id,
            email: "test@test.com".to_string(),
            role: role.to_string(),
            organization_id: None,
        }
    }

    fn make_subscription(user_id: Uuid, status: &str) -> Subscription {
        Subscription {
            id: Uuid::new_v4(),
            user_id,
            plan_id: Uuid::new_v4(),
            locked_price_id: None,
            stripe_subscription_id: "sub_test".to_string(),
            status: status.to_string(),
            trial_ends_at: None,
            current_period_start: None,
            current_period_end: None,
            cancel_at_period_end: false,
            canceled_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn member_can_edit_own_profile() {
        let user_id = Uuid::new_v4();
        let user = make_user(user_id, "member");
        assert!(can_edit_profile(&user, user_id, false));
    }

    #[test]
    fn member_cannot_edit_other_profile() {
        let user = make_user(Uuid::new_v4(), "member");
        let other_id = Uuid::new_v4();
        assert!(!can_edit_profile(&user, other_id, false));
    }

    #[test]
    fn admin_can_edit_non_owner_profile() {
        let admin = make_user(Uuid::new_v4(), "admin");
        let other_id = Uuid::new_v4();
        assert!(can_edit_profile(&admin, other_id, false));
    }

    #[test]
    fn admin_cannot_edit_org_owner_profile() {
        let admin = make_user(Uuid::new_v4(), "admin");
        let owner_id = Uuid::new_v4();
        assert!(!can_edit_profile(&admin, owner_id, true));
    }

    #[test]
    fn cannot_cancel_already_canceled_subscription() {
        let user_id = Uuid::new_v4();
        let user = make_user(user_id, "member");
        let sub = make_subscription(user_id, "canceled");
        assert!(!can_cancel_subscription(&user, &sub));
    }

    #[test]
    fn member_can_cancel_own_subscription() {
        let user_id = Uuid::new_v4();
        let user = make_user(user_id, "member");
        let sub = make_subscription(user_id, "active");
        assert!(can_cancel_subscription(&user, &sub));
    }

    #[test]
    fn member_cannot_cancel_others_subscription() {
        let user = make_user(Uuid::new_v4(), "member");
        let sub = make_subscription(Uuid::new_v4(), "active");
        assert!(!can_cancel_subscription(&user, &sub));
    }
}
