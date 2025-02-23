-- This file should undo anything in `up.sql`
-- create_workflows_executions_users/up.sql

-- ========================================
-- 1) 创建 users 表
-- ========================================
CREATE TABLE IF NOT EXISTS "users" (
  "id" TEXT PRIMARY KEY,
  "email" TEXT NOT NULL UNIQUE,
  "password_hash" TEXT NOT NULL,
  "role" TEXT NOT NULL DEFAULT 'member', -- 'member' / 'admin' / etc
  "created_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  "updated_at" TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

-- ========================================
-- 2) 创建 workflows 表
-- ========================================
CREATE TABLE IF NOT EXISTS "workflows" (
  "id" TEXT PRIMARY KEY,
  "name" TEXT NOT NULL,
  "active" BOOLEAN NOT NULL DEFAULT 0,
  "nodes" TEXT NOT NULL,         -- JSON: Node definitions
  "connections" TEXT NOT NULL,   -- JSON: Connections info
  "settings" TEXT,               -- JSON: optional
  "static_data" TEXT,            -- JSON: optional
  "meta" TEXT,                   -- JSON: optional
  "owner_id" TEXT,               -- FK to users.id (可为null)
  "created_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  "updated_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY("owner_id") REFERENCES "users"("id") ON DELETE SET NULL
);

-- ========================================
-- 3) 创建 executions 表
-- ========================================
CREATE TABLE IF NOT EXISTS "executions" (
  "id" TEXT PRIMARY KEY,
  "workflow_id" TEXT NOT NULL,
  "finished" BOOLEAN NOT NULL DEFAULT 0,
  "mode" TEXT NOT NULL DEFAULT 'manual',
  "started_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  "stopped_at" TIMESTAMP,
  "data" TEXT,                      -- JSON: execution logs/outputs
  "started_by_user_id" TEXT,        -- FK to users.id
  "created_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  "updated_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY("workflow_id") REFERENCES "workflows"("id") ON DELETE CASCADE,
  FOREIGN KEY("started_by_user_id") REFERENCES "users"("id") ON DELETE SET NULL
);

-- ========================================
-- 4) 创建 shared_workflows 表
-- ========================================
-- 用于多用户协作: userId 与 workflowId 的多对多
-- permission: 'owner' / 'editor' / 'viewer' 等
CREATE TABLE IF NOT EXISTS "shared_workflows" (
  "id" TEXT PRIMARY KEY,
  "workflow_id" TEXT NOT NULL,
  "user_id" TEXT NOT NULL,
  "permission" TEXT NOT NULL DEFAULT 'editor',  -- e.g. 'owner', 'editor', 'viewer'
  "created_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  "updated_at" TIMESTAMP NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY("workflow_id") REFERENCES "workflows"("id") ON DELETE CASCADE,
  FOREIGN KEY("user_id") REFERENCES "users"("id") ON DELETE CASCADE
);-- Your SQL goes here
