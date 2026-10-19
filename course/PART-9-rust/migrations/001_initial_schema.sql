-- Initial schema for the SaaS backend
-- Run with: sqlx migrate run
-- Or automatically on startup via: sqlx::migrate!("./migrations").run(&pool).await?

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================
-- RBAC: Roles and Permissions
-- ============================================================

CREATE TABLE roles (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE permissions (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE role_permissions (
    role_id       UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- Seed roles
INSERT INTO roles (name, description) VALUES
    ('admin',   'Full access to all resources'),
    ('member',  'Standard user access'),
    ('guest',   'Read-only access'),
    ('support', 'Customer support access');

-- Seed permissions
INSERT INTO permissions (name, description) VALUES
    ('users:read',              'View user profiles'),
    ('users:write',             'Create and update users'),
    ('users:delete',            'Delete users'),
    ('products:read',           'View products'),
    ('products:write',          'Create and update products'),
    ('products:delete',         'Delete products'),
    ('prices:write',            'Add and update prices'),
    ('subscriptions:read_own',  'View own subscription'),
    ('subscriptions:read_all',  'View all subscriptions'),
    ('subscriptions:manage',    'Create, update, cancel subscriptions'),
    ('coupons:manage',          'Create and manage coupons'),
    ('billing:read',            'View billing information'),
    ('billing:manage',          'Manage billing settings');

-- Admin gets all permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'admin';

-- Member permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'member'
  AND p.name IN (
    'users:read', 'products:read',
    'subscriptions:read_own', 'subscriptions:manage',
    'billing:read'
  );

-- Guest permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'guest'
  AND p.name IN ('products:read');

-- Support permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'support'
  AND p.name IN ('users:read', 'subscriptions:read_all', 'billing:read');

-- ============================================================
-- Users
-- ============================================================

CREATE TABLE users (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email               TEXT NOT NULL UNIQUE,
    password_hash       TEXT NOT NULL,
    display_name        TEXT,
    role                TEXT NOT NULL DEFAULT 'member' REFERENCES roles(name),
    stripe_customer_id  TEXT UNIQUE,
    organization_id     UUID,    -- FK added after organizations table
    is_org_owner        BOOLEAN NOT NULL DEFAULT FALSE,
    email_verified_at   TIMESTAMPTZ,
    last_login_at       TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;

-- ============================================================
-- Organizations (Multi-Tenancy)
-- ============================================================

CREATE TABLE organizations (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    owner_id    UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);

ALTER TABLE users
    ADD CONSTRAINT fk_users_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id);

CREATE TABLE organization_members (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member',
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, user_id)
);

CREATE INDEX idx_org_members_user_id ON organization_members(user_id);

-- ============================================================
-- Products and Pricing
-- ============================================================

CREATE TABLE products (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                TEXT NOT NULL,
    description         TEXT,
    stripe_product_id   TEXT UNIQUE,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

-- Prices for one-time products (price history with is_current flag)
CREATE TABLE prices (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id          UUID REFERENCES products(id) ON DELETE CASCADE,
    plan_id             UUID,           -- FK to plans added below
    stripe_price_id     TEXT UNIQUE,
    amount_cents        INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency            TEXT NOT NULL DEFAULT 'usd',
    is_current          BOOLEAN NOT NULL DEFAULT TRUE,
    applies_to          TEXT NOT NULL DEFAULT 'future_members'
                        CHECK (applies_to IN ('future_members', 'all_members', 'from_date')),
    applies_from_date   DATE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prices_product_id ON prices(product_id);
CREATE INDEX idx_prices_plan_id ON prices(plan_id);
CREATE INDEX idx_prices_current ON prices(product_id) WHERE is_current = TRUE;

-- ============================================================
-- Plans (Subscription Tiers)
-- ============================================================

CREATE TABLE plans (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                TEXT NOT NULL,
    description         TEXT,
    interval            TEXT NOT NULL CHECK (interval IN ('month', 'year')),
    interval_count      INTEGER NOT NULL DEFAULT 1 CHECK (interval_count > 0),
    stripe_product_id   TEXT UNIQUE,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

-- Add FK from prices to plans now that plans table exists
ALTER TABLE prices
    ADD CONSTRAINT fk_prices_plan
    FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE;

-- ============================================================
-- Subscriptions
-- ============================================================

CREATE TABLE subscriptions (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                 UUID NOT NULL REFERENCES users(id),
    organization_id         UUID REFERENCES organizations(id),
    plan_id                 UUID NOT NULL REFERENCES plans(id),
    locked_price_id         UUID REFERENCES prices(id),
    stripe_subscription_id  TEXT UNIQUE,
    status                  TEXT NOT NULL DEFAULT 'trialing'
                            CHECK (status IN ('trialing', 'active', 'past_due', 'canceled', 'unpaid', 'incomplete')),
    trial_ends_at           TIMESTAMPTZ,
    current_period_start    TIMESTAMPTZ,
    current_period_end      TIMESTAMPTZ,
    cancel_at_period_end    BOOLEAN NOT NULL DEFAULT FALSE,
    canceled_at             TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);
CREATE INDEX idx_subscriptions_stripe_id ON subscriptions(stripe_subscription_id);

-- ============================================================
-- Coupons
-- ============================================================

CREATE TABLE coupons (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code            TEXT NOT NULL UNIQUE,
    discount_type   TEXT NOT NULL CHECK (discount_type IN ('percent', 'fixed')),
    discount_value  INTEGER NOT NULL CHECK (discount_value > 0),
    max_uses        INTEGER,
    current_uses    INTEGER NOT NULL DEFAULT 0,
    expires_at      TIMESTAMPTZ,
    applies_to      TEXT NOT NULL DEFAULT 'any',
    stripe_coupon_id TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE coupon_redemptions (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    coupon_id   UUID NOT NULL REFERENCES coupons(id),
    user_id     UUID NOT NULL REFERENCES users(id),
    redeemed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(coupon_id, user_id)
);

-- ============================================================
-- Invoices and Payments
-- ============================================================

CREATE TABLE invoices (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id),
    subscription_id     UUID REFERENCES subscriptions(id),
    stripe_invoice_id   TEXT UNIQUE,
    amount_cents        INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency            TEXT NOT NULL DEFAULT 'usd',
    status              TEXT NOT NULL CHECK (status IN ('draft', 'open', 'paid', 'void', 'uncollectible')),
    paid_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE payments (
    id                          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                     UUID NOT NULL REFERENCES users(id),
    invoice_id                  UUID REFERENCES invoices(id),
    stripe_payment_intent_id    TEXT UNIQUE,
    amount_cents                INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency                    TEXT NOT NULL DEFAULT 'usd',
    status                      TEXT NOT NULL,
    idempotency_key             TEXT UNIQUE,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Webhook Events (Idempotency Log)
-- ============================================================

CREATE TABLE webhook_events (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_event_id     TEXT NOT NULL UNIQUE,
    event_type          TEXT NOT NULL,
    payload             JSONB NOT NULL,
    processed_at        TIMESTAMPTZ,
    error               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_events_stripe_id ON webhook_events(stripe_event_id);
CREATE INDEX idx_webhook_events_unprocessed ON webhook_events(created_at) WHERE processed_at IS NULL;

-- ============================================================
-- Audit Logs
-- ============================================================

CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    actor_id        UUID REFERENCES users(id),
    actor_type      TEXT NOT NULL DEFAULT 'user',
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     UUID,
    metadata        JSONB,
    ip_address      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
