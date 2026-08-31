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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account TEXT NOT NULL,
            code TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source TEXT NOT NULL,
            UNIQUE(account, code, timestamp)
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert(pool: &SqlitePool, rec: &CodeRecord) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT OR IGNORE INTO codes (account, code, timestamp, source) VALUES (?, ?, ?, ?)",
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
        "SELECT account, code, timestamp, source FROM codes ORDER BY id DESC LIMIT ?",
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
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got, vec![rec()]);
    }

    #[tokio::test]
    async fn duplicate_is_ignored() {
        let pool = pool().await;
        assert!(insert(&pool, &rec()).await.unwrap());
        assert!(!insert(&pool, &rec()).await.unwrap());
        assert_eq!(recent(&pool, 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn recent_is_newest_first() {
        let pool = pool().await;
        let mut a = rec();
        a.timestamp = "2026-08-31T12:00:00Z".into();
        let mut b = rec();
        b.timestamp = "2026-08-31T12:00:30Z".into();
        insert(&pool, &a).await.unwrap();
        insert(&pool, &b).await.unwrap();
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got[0].timestamp, "2026-08-31T12:00:30Z");
    }
}
