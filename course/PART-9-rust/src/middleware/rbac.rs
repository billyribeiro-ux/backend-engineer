use crate::{auth::extractor::AuthUser, errors::AppError};

/// Check if a user has a specific role.
/// Used inside handlers as a guard.
pub fn require_role(user: &AuthUser, allowed_roles: &[&str]) -> Result<(), AppError> {
    if allowed_roles.contains(&user.role.as_str()) {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "You do not have the required role to perform this action.",
        ))
    }
}

/// Check if a user's role has a specific permission.
/// This requires a database query to check the role_permissions table.
pub async fn check_permission(
    db: &sqlx::PgPool,
    role: &str,
    permission: &str,
) -> Result<bool, AppError> {
    let has_permission = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM role_permissions rp
            JOIN roles r ON r.id = rp.role_id
            JOIN permissions p ON p.id = rp.permission_id
            WHERE r.name = $1 AND p.name = $2
        ) as "has_permission!"
        "#,
        role,
        permission
    )
    .fetch_one(db)
    .await
    .map_err(AppError::Database)?;

    Ok(has_permission)
}

/// Guard helper for handlers: check permission and return AppError if denied.
pub async fn require_permission(
    db: &sqlx::PgPool,
    user: &AuthUser,
    permission: &str,
) -> Result<(), AppError> {
    let has = check_permission(db, &user.role, permission).await?;
    if has {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "You do not have permission to perform this action.",
        ))
    }
}
