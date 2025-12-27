use std::env;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::app::{commands, queries};
use crate::data::task_repo;
use crate::domain::principal::Principal;
use crate::domain::task::TaskStatus;

static MIGRATED: OnceCell<()> = OnceCell::const_new();

async fn test_pool() -> Option<PgPool> {
    let url = env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    ensure_migrated(&pool).await;
    Some(pool)
}

async fn ensure_migrated(pool: &PgPool) {
    MIGRATED
        .get_or_init(|| async {
            sqlx::migrate!("./migrations")
                .run(pool)
                .await
                .expect("migrations succeed");
        })
        .await;
}

fn test_principal() -> Principal {
    Principal {
        subject: "test-user".to_string(),
        display_name: "Test User".to_string(),
        email: Some("test@example.com".to_string()),
        tenant_id: None,
        roles: Vec::new(),
    }
}

async fn cleanup_tasks(pool: &PgPool, ids: &[Uuid]) {
    for id in ids {
        let _ = sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }
}

#[tokio::test]
async fn create_start_complete_happy_path() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: "spec-test-happy".to_string(),
        description_raw: "spec-test-happy".to_string(),
        due_at: None,
        priority_raw: Some(3),
    };
    let id = commands::create_task::handle(&pool, &principal, cmd)
        .await
        .expect("create task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    assert_eq!(row.status, "PLANNED");
    let start = commands::start_task::StartTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::start_task::handle(&pool, &principal, start)
        .await
        .expect("start task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    assert_eq!(row.status, "IN_PROGRESS");
    let complete = commands::complete_task::CompleteTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::complete_task::handle(&pool, &principal, complete)
        .await
        .expect("complete task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    assert_eq!(row.status, "COMPLETED");
    cleanup_tasks(&pool, &[id]).await;
}

#[tokio::test]
async fn complete_rejects_planned() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: "spec-test-invalid".to_string(),
        description_raw: "spec-test-invalid".to_string(),
        due_at: None,
        priority_raw: Some(3),
    };
    let id = commands::create_task::handle(&pool, &principal, cmd)
        .await
        .expect("create task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let complete = commands::complete_task::CompleteTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    let result = commands::complete_task::handle(&pool, &principal, complete).await;
    assert!(matches!(
        result,
        Err(commands::complete_task::CompleteTaskError::InvalidTransition)
    ));
    cleanup_tasks(&pool, &[id]).await;
}

#[tokio::test]
async fn update_rejects_completed_task() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: "spec-test-completed".to_string(),
        description_raw: "spec-test-completed".to_string(),
        due_at: None,
        priority_raw: Some(3),
    };
    let id = commands::create_task::handle(&pool, &principal, cmd)
        .await
        .expect("create task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let start = commands::start_task::StartTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::start_task::handle(&pool, &principal, start)
        .await
        .expect("start task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let complete = commands::complete_task::CompleteTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::complete_task::handle(&pool, &principal, complete)
        .await
        .expect("complete task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let update = commands::update_task_details::UpdateTaskDetailsCommand {
        id,
        title_raw: "spec-test-completed-updated".to_string(),
        description_raw: "spec-test-completed-updated".to_string(),
        due_at: None,
        priority_raw: 2,
        expected_row_version: row.row_version,
    };
    let result = commands::update_task_details::handle(&pool, &principal, update).await;
    assert!(matches!(
        result,
        Err(commands::update_task_details::UpdateTaskDetailsError::Completed)
    ));
    cleanup_tasks(&pool, &[id]).await;
}

#[tokio::test]
async fn update_detects_concurrency_conflict() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: "spec-test-concurrency".to_string(),
        description_raw: "spec-test-concurrency".to_string(),
        due_at: None,
        priority_raw: Some(3),
    };
    let id = commands::create_task::handle(&pool, &principal, cmd)
        .await
        .expect("create task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let update = commands::update_task_details::UpdateTaskDetailsCommand {
        id,
        title_raw: "spec-test-concurrency-1".to_string(),
        description_raw: "spec-test-concurrency-1".to_string(),
        due_at: None,
        priority_raw: 4,
        expected_row_version: row.row_version,
    };
    commands::update_task_details::handle(&pool, &principal, update)
        .await
        .expect("first update");
    let stale_update = commands::update_task_details::UpdateTaskDetailsCommand {
        id,
        title_raw: "spec-test-concurrency-2".to_string(),
        description_raw: "spec-test-concurrency-2".to_string(),
        due_at: None,
        priority_raw: 4,
        expected_row_version: row.row_version,
    };
    let result = commands::update_task_details::handle(&pool, &principal, stale_update).await;
    assert!(matches!(
        result,
        Err(commands::update_task_details::UpdateTaskDetailsError::ConcurrencyConflict)
    ));
    cleanup_tasks(&pool, &[id]).await;
}

#[tokio::test]
async fn delete_is_idempotent_and_queries_exclude_deleted() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let cmd = commands::create_task::CreateTaskCommand {
        title_raw: "spec-test-delete".to_string(),
        description_raw: "spec-test-delete".to_string(),
        due_at: None,
        priority_raw: Some(3),
    };
    let id = commands::create_task::handle(&pool, &principal, cmd)
        .await
        .expect("create task");
    let row = task_repo::fetch_task(&pool, id)
        .await
        .unwrap()
        .expect("task row");
    let delete = commands::delete_task::DeleteTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::delete_task::handle(&pool, &principal, delete)
        .await
        .expect("delete task");
    let delete_again = commands::delete_task::DeleteTaskCommand {
        id,
        expected_row_version: row.row_version,
    };
    commands::delete_task::handle(&pool, &principal, delete_again)
        .await
        .expect("delete task again");
    let list = queries::list_tasks::handle(
        &pool,
        &principal,
        queries::list_tasks::ListTasksQuery {
            status: None,
            created_after: None,
            created_before: None,
            search: Some("spec-test-delete".to_string()),
            priority: None,
            sort: None,
            limit: 10,
        },
    )
    .await
    .expect("list tasks");
    assert!(list.is_empty());
    let result = queries::get_task::handle(&pool, &principal, id).await;
    assert!(matches!(
        result,
        Err(queries::get_task::GetTaskError::NotFound)
    ));
    cleanup_tasks(&pool, &[id]).await;
}

#[tokio::test]
async fn list_filters_and_sorting() {
    let Some(pool) = test_pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let principal = test_principal();
    let mut ids = Vec::new();
    let now = Utc::now();
    for suffix in ["a", "b", "c"] {
        let cmd = commands::create_task::CreateTaskCommand {
            title_raw: format!("spec-test-list-{suffix}"),
            description_raw: format!("spec-test-list-{suffix}"),
            due_at: None,
            priority_raw: Some(3),
        };
        let id = commands::create_task::handle(&pool, &principal, cmd)
            .await
            .expect("create task");
        ids.push(id);
    }
    let (id_a, id_b, id_c) = (ids[0], ids[1], ids[2]);
    let row_a = task_repo::fetch_task(&pool, id_a).await.unwrap().unwrap();
    let row_b = task_repo::fetch_task(&pool, id_b).await.unwrap().unwrap();
    let row_c = task_repo::fetch_task(&pool, id_c).await.unwrap().unwrap();
    commands::update_task_details::handle(
        &pool,
        &principal,
        commands::update_task_details::UpdateTaskDetailsCommand {
            id: id_a,
            title_raw: "spec-test-list-a".to_string(),
            description_raw: "spec-test-list-a".to_string(),
            due_at: Some(now + Duration::days(2)),
            priority_raw: 5,
            expected_row_version: row_a.row_version,
        },
    )
    .await
    .expect("update task a");
    commands::update_task_details::handle(
        &pool,
        &principal,
        commands::update_task_details::UpdateTaskDetailsCommand {
            id: id_b,
            title_raw: "spec-test-list-b".to_string(),
            description_raw: "spec-test-list-b".to_string(),
            due_at: Some(now + Duration::days(1)),
            priority_raw: 2,
            expected_row_version: row_b.row_version,
        },
    )
    .await
    .expect("update task b");
    commands::update_task_details::handle(
        &pool,
        &principal,
        commands::update_task_details::UpdateTaskDetailsCommand {
            id: id_c,
            title_raw: "spec-test-list-c".to_string(),
            description_raw: "spec-test-list-c".to_string(),
            due_at: Some(now + Duration::days(3)),
            priority_raw: 3,
            expected_row_version: row_c.row_version,
        },
    )
    .await
    .expect("update task c");
    let row_b = task_repo::fetch_task(&pool, id_b).await.unwrap().unwrap();
    commands::start_task::handle(
        &pool,
        &principal,
        commands::start_task::StartTaskCommand {
            id: id_b,
            expected_row_version: row_b.row_version,
        },
    )
    .await
    .expect("start task b");
    let row_c = task_repo::fetch_task(&pool, id_c).await.unwrap().unwrap();
    commands::start_task::handle(
        &pool,
        &principal,
        commands::start_task::StartTaskCommand {
            id: id_c,
            expected_row_version: row_c.row_version,
        },
    )
    .await
    .expect("start task c");
    let row_c = task_repo::fetch_task(&pool, id_c).await.unwrap().unwrap();
    commands::complete_task::handle(
        &pool,
        &principal,
        commands::complete_task::CompleteTaskCommand {
            id: id_c,
            expected_row_version: row_c.row_version,
        },
    )
    .await
    .expect("complete task c");
    let older = now - Duration::days(2);
    sqlx::query("UPDATE tasks SET created_at = $1 WHERE id = $2")
        .bind(older)
        .bind(id_a)
        .execute(&pool)
        .await
        .expect("adjust created_at");
    let completed = queries::list_tasks::handle(
        &pool,
        &principal,
        queries::list_tasks::ListTasksQuery {
            status: Some(TaskStatus::Completed),
            created_after: None,
            created_before: None,
            search: Some("spec-test-list".to_string()),
            priority: None,
            sort: None,
            limit: 10,
        },
    )
    .await
    .expect("list completed");
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, id_c);
    let recent = queries::list_tasks::handle(
        &pool,
        &principal,
        queries::list_tasks::ListTasksQuery {
            status: None,
            created_after: Some(now - Duration::days(1)),
            created_before: None,
            search: Some("spec-test-list".to_string()),
            priority: None,
            sort: None,
            limit: 10,
        },
    )
    .await
    .expect("list recent");
    assert!(!recent.iter().any(|task| task.id == id_a));
    let due_sorted = queries::list_tasks::handle(
        &pool,
        &principal,
        queries::list_tasks::ListTasksQuery {
            status: None,
            created_after: None,
            created_before: None,
            search: Some("spec-test-list".to_string()),
            priority: None,
            sort: Some("due_at".to_string()),
            limit: 10,
        },
    )
    .await
    .expect("list due sorted");
    let due_ids: Vec<Uuid> = due_sorted.iter().map(|task| task.id).collect();
    assert_eq!(due_ids, vec![id_b, id_a, id_c]);
    let priority_sorted = queries::list_tasks::handle(
        &pool,
        &principal,
        queries::list_tasks::ListTasksQuery {
            status: None,
            created_after: None,
            created_before: None,
            search: Some("spec-test-list".to_string()),
            priority: None,
            sort: Some("priority".to_string()),
            limit: 10,
        },
    )
    .await
    .expect("list priority sorted");
    let priority_ids: Vec<Uuid> = priority_sorted.iter().map(|task| task.id).collect();
    assert_eq!(priority_ids, vec![id_b, id_c, id_a]);
    cleanup_tasks(&pool, &ids).await;
}
