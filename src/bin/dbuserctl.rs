use anyhow::{anyhow, Context};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use rand::rngs::OsRng;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
    email: String,
    display_name: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
struct Args {
    command: String,
    help: bool,
    id: Option<Uuid>,
    email: Option<String>,
    password: Option<String>,
    display_name: Option<String>,
    is_active: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = parse_args(std::env::args().skip(1))?;
    if args.help || args.command == "help" {
        println!("{}", usage());
        return Ok(());
    }
    let database_url = std::env::var("DATABASE_URL").context("missing env var DATABASE_URL")?;
    let pool = PgPool::connect(&database_url)
        .await
        .context("connecting to database")?;

    match args.command.as_str() {
        "view" => cmd_view(&pool, &args).await?,
        "list" => cmd_list(&pool).await?,
        "insert" => cmd_insert(&pool, &args).await?,
        "update" => cmd_update(&pool, &args).await?,
        "delete" => cmd_delete(&pool, &args).await?,
        _ => return Err(anyhow!("unknown command: {}", args.command)),
    }
    Ok(())
}

async fn cmd_list(pool: &PgPool) -> Result<(), anyhow::Error> {
    let rows = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, is_active, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        println!("no users");
        return Ok(());
    }

    for u in rows {
        println!(
            "{} | {} | {} | active={} | created_at={} | updated_at={}",
            u.id, u.email, u.display_name, u.is_active, u.created_at, u.updated_at
        );
    }
    Ok(())
}

async fn cmd_view(pool: &PgPool, args: &Args) -> Result<(), anyhow::Error> {
    let user = if let Some(id) = args.id {
        fetch_user_by_id(pool, id).await?
    } else if let Some(email) = args.email.as_deref() {
        fetch_user_by_email(pool, email).await?
    } else {
        return Err(anyhow!("view requires --id or --email"));
    };

    match user {
        Some(u) => {
            println!("id: {}", u.id);
            println!("email: {}", u.email);
            println!("display_name: {}", u.display_name);
            println!("is_active: {}", u.is_active);
            println!("created_at: {}", u.created_at);
            println!("updated_at: {}", u.updated_at);
        }
        None => {
            println!("user not found");
        }
    }
    Ok(())
}

async fn cmd_delete(pool: &PgPool, args: &Args) -> Result<(), anyhow::Error> {
    let deleted = if let Some(id) = args.id {
        sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
    } else if let Some(email) = args.email.as_deref() {
        sqlx::query(
            r#"
            DELETE FROM users
            WHERE lower(email) = lower($1)
            "#,
        )
        .bind(email)
        .execute(pool)
        .await?
        .rows_affected()
    } else {
        return Err(anyhow!("delete requires --id or --email"));
    };

    if deleted == 0 {
        println!("no user deleted");
    } else {
        println!("deleted {} user(s)", deleted);
    }
    Ok(())
}

async fn cmd_insert(pool: &PgPool, args: &Args) -> Result<(), anyhow::Error> {
    let email = args
        .email
        .as_deref()
        .ok_or_else(|| anyhow!("insert requires --email"))?;
    let display_name = args
        .display_name
        .as_deref()
        .ok_or_else(|| anyhow!("insert requires --display-name"))?;
    let password = args
        .password
        .as_deref()
        .ok_or_else(|| anyhow!("insert requires --password"))?;

    let password_hash = hash_password(password)?;
    let is_active = args.is_active.unwrap_or(true);

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, display_name, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, now(), now())
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(is_active)
    .fetch_one(pool)
    .await?;

    println!("inserted user id: {}", id);
    Ok(())
}

async fn cmd_update(pool: &PgPool, args: &Args) -> Result<(), anyhow::Error> {
    let (id, email) = match (args.id, args.email.as_deref()) {
        (Some(id), _) => (Some(id), None),
        (None, Some(email)) => (None, Some(email)),
        _ => return Err(anyhow!("update requires --id or --email")),
    };

    let mut new_password_hash = None;
    if let Some(password) = args.password.as_deref() {
        new_password_hash = Some(hash_password(password)?);
    }

    let updated = if let Some(id) = id {
        sqlx::query(
            r#"
            UPDATE users
            SET email = COALESCE($2, email),
                password_hash = COALESCE($3, password_hash),
                display_name = COALESCE($4, display_name),
                is_active = COALESCE($5, is_active),
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(args.email.clone())
        .bind(new_password_hash.clone())
        .bind(args.display_name.clone())
        .bind(args.is_active)
        .execute(pool)
        .await?
        .rows_affected()
    } else {
        sqlx::query(
            r#"
            UPDATE users
            SET email = COALESCE($2, email),
                password_hash = COALESCE($3, password_hash),
                display_name = COALESCE($4, display_name),
                is_active = COALESCE($5, is_active),
                updated_at = now()
            WHERE lower(email) = lower($1)
            "#,
        )
        .bind(email.unwrap())
        .bind(args.email.clone())
        .bind(new_password_hash.clone())
        .bind(args.display_name.clone())
        .bind(args.is_active)
        .execute(pool)
        .await?
        .rows_affected()
    };

    if updated == 0 {
        println!("no user updated");
    } else {
        println!("updated {} user(s)", updated);
    }
    Ok(())
}

async fn fetch_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRecord>, anyhow::Error> {
    let row = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

async fn fetch_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserRecord>, anyhow::Error> {
    let row = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, is_active, created_at, updated_at
        FROM users
        WHERE lower(email) = lower($1)
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

fn hash_password(password: &str) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow!("failed to hash password"))?
        .to_string();
    Ok(hash)
}

fn parse_args<I>(mut args: I) -> Result<Args, anyhow::Error>
where
    I: Iterator<Item = String>,
{
    let command = match args.next() {
        Some(cmd) => cmd,
        None => {
            return Ok(Args {
                command: "help".to_string(),
                help: true,
                id: None,
                email: None,
                password: None,
                display_name: None,
                is_active: None,
            });
        }
    };
    let mut parsed = Args {
        command,
        help: false,
        id: None,
        email: None,
        password: None,
        display_name: None,
        is_active: None,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--id" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("--id requires a value"))?;
                parsed.id = Some(Uuid::parse_str(&value).context("invalid --id")?);
            }
            "--email" => {
                parsed.email = Some(
                    args.next()
                        .ok_or_else(|| anyhow!("--email requires a value"))?,
                );
            }
            "--password" => {
                parsed.password = Some(
                    args.next()
                        .ok_or_else(|| anyhow!("--password requires a value"))?,
                );
            }
            "--display-name" => {
                parsed.display_name = Some(
                    args.next()
                        .ok_or_else(|| anyhow!("--display-name requires a value"))?,
                );
            }
            "--is-active" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("--is-active requires a value"))?;
                parsed.is_active = Some(parse_bool(&value)?);
            }
            "--help" | "-h" => {
                parsed.help = true;
            }
            _ => return Err(anyhow!("unknown argument: {}", arg)),
        }
    }

    Ok(parsed)
}

fn parse_bool(raw: &str) -> Result<bool, anyhow::Error> {
    match raw.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => Err(anyhow!("invalid boolean value: {}", raw)),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  dbuserctl help | --help | -h",
        "  dbuserctl list",
        "  dbuserctl view --id <uuid> | --email <email>",
        "  dbuserctl insert --email <email> --display-name <name> --password <password> [--is-active true|false]",
        "  dbuserctl update --id <uuid> | --email <email> [--email <email>] [--display-name <name>] [--password <password>] [--is-active true|false]",
        "  dbuserctl delete --id <uuid> | --email <email>",
        "",
        "Environment:",
        "  DATABASE_URL must be set",
    ]
    .join("\n")
}
