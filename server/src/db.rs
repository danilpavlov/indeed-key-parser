use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct CodeRecord {
    pub account: String,
    pub code: String,
    pub timestamp: String,
    pub source: String,
}

pub async fn init(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS codes (
            account TEXT PRIMARY KEY,
            code TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    migrate_legacy(pool).await?;
    Ok(())
}

/// Collapse a pre-existing multi-row table (the old layout keyed by an
/// autoincrement `id`, one row per received code) down to the latest code per
/// account, and rebuild it with `account` as the primary key.
async fn migrate_legacy(pool: &SqlitePool) -> sqlx::Result<()> {
    let has_id: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_info('codes') WHERE name = 'id'")
            .fetch_one(pool)
            .await?;
    if has_id == 0 {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    sqlx::query(
        "CREATE TABLE codes_new (
            account TEXT PRIMARY KEY,
            code TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source TEXT NOT NULL
        )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO codes_new (account, code, timestamp, source)
         SELECT account, code, timestamp, source FROM codes
         WHERE id IN (SELECT MAX(id) FROM codes GROUP BY account)",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DROP TABLE codes").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE codes_new RENAME TO codes")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Store the code as the account's single latest entry. An incoming code that is
/// older than the stored one (a late retry) is ignored. Returns whether a row
/// was written.
pub async fn insert(pool: &SqlitePool, rec: &CodeRecord) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO codes (account, code, timestamp, source)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(account) DO UPDATE SET
             code = excluded.code,
             timestamp = excluded.timestamp,
             source = excluded.source
         WHERE datetime(excluded.timestamp) >= datetime(codes.timestamp)",
    )
    .bind(&rec.account)
    .bind(&rec.code)
    .bind(&rec.timestamp)
    .bind(&rec.source)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn recent(pool: &SqlitePool, limit: i64) -> sqlx::Result<Vec<CodeRecord>> {
    sqlx::query_as::<_, CodeRecord>(
        "SELECT account, code, timestamp, source FROM codes ORDER BY datetime(timestamp) DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&pool).await.unwrap();
        pool
    }

    fn rec() -> CodeRecord {
        CodeRecord {
            account: "Corp".into(),
            code: "123456".into(),
            timestamp: "2026-08-31T12:00:00Z".into(),
            source: "com.indeedid.key".into(),
        }
    }

    #[tokio::test]
    async fn insert_then_recent_returns_it() {
        let pool = pool().await;
        assert!(insert(&pool, &rec()).await.unwrap());
        assert_eq!(recent(&pool, 10).await.unwrap(), vec![rec()]);
    }

    #[tokio::test]
    async fn newer_code_replaces_the_account_row() {
        let pool = pool().await;
        insert(&pool, &rec()).await.unwrap();
        let mut newer = rec();
        newer.code = "654321".into();
        newer.timestamp = "2026-08-31T12:00:30Z".into();
        assert!(insert(&pool, &newer).await.unwrap());
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got.len(), 1); // one row per account
        assert_eq!(got[0].code, "654321"); // the freshest
    }

    #[tokio::test]
    async fn stale_code_does_not_overwrite_newer() {
        let pool = pool().await;
        let mut newer = rec();
        newer.code = "654321".into();
        newer.timestamp = "2026-08-31T12:00:30Z".into();
        insert(&pool, &newer).await.unwrap();
        // a delayed older code for the same account arrives late
        assert!(!insert(&pool, &rec()).await.unwrap());
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].code, "654321");
    }

    #[tokio::test]
    async fn each_account_keeps_its_own_latest() {
        let pool = pool().await;
        let mut b = rec();
        b.account = "Bank".into();
        b.code = "999999".into();
        b.timestamp = "2026-08-31T12:01:00Z".into();
        insert(&pool, &rec()).await.unwrap();
        insert(&pool, &b).await.unwrap();
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].account, "Bank"); // newest-updated first
    }

    #[tokio::test]
    async fn migrates_a_legacy_multi_row_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account TEXT NOT NULL, code TEXT NOT NULL,
                timestamp TEXT NOT NULL, source TEXT NOT NULL,
                UNIQUE(account, code, timestamp))",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (c, t) in [
            ("111111", "2026-08-31T12:00:00Z"),
            ("222222", "2026-08-31T12:00:30Z"),
        ] {
            sqlx::query(
                "INSERT INTO codes (account, code, timestamp, source) VALUES ('Corp', ?, ?, 'src')",
            )
            .bind(c)
            .bind(t)
            .execute(&pool)
            .await
            .unwrap();
        }
        init(&pool).await.unwrap(); // migrates
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].code, "222222"); // latest kept
        // behaves as one-row-per-account afterwards
        let newer = CodeRecord {
            account: "Corp".into(),
            code: "333333".into(),
            timestamp: "2026-08-31T12:01:00Z".into(),
            source: "src".into(),
        };
        insert(&pool, &newer).await.unwrap();
        assert_eq!(recent(&pool, 10).await.unwrap().len(), 1);
    }
}
