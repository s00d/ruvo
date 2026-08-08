//! App CLI: `migrate [up|down|status] [N]`.

use sova_core::Error;
use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;

const USAGE: &str = "usage: migrate [up|down|status] [N]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrateCmd {
    /// Apply pending migrations (`None` = all).
    Up(Option<u32>),
    /// Roll back `n` applied migrations (default 1 at the CLI layer).
    Down(u32),
    Status,
}

/// Parse argv after the `migrate` command name.
pub fn parse_migrate_args(args: &[String]) -> Result<MigrateCmd, String> {
    match args {
        [] => Ok(MigrateCmd::Up(None)),
        [s] if s == "up" => Ok(MigrateCmd::Up(None)),
        [s] if s == "down" => Ok(MigrateCmd::Down(1)),
        [s] if s == "status" => Ok(MigrateCmd::Status),
        [s, n] if s == "up" => Ok(MigrateCmd::Up(Some(parse_steps(n)?))),
        [s, n] if s == "down" => Ok(MigrateCmd::Down(parse_steps(n)?)),
        _ => Err(USAGE.into()),
    }
}

fn parse_steps(raw: &str) -> Result<u32, String> {
    let n: u32 = raw
        .parse()
        .map_err(|_| format!("invalid step count `{raw}`; {USAGE}"))?;
    if n == 0 {
        return Err(format!("step count must be >= 1; {USAGE}"));
    }
    Ok(n)
}

pub(crate) async fn run_migrate<M: MigratorTrait>(
    conn: DatabaseConnection,
    args: &[String],
) -> Result<(), Error> {
    let cmd = parse_migrate_args(args).map_err(Error::Internal)?;
    match cmd {
        MigrateCmd::Up(steps) => M::up(&conn, steps)
            .await
            .map_err(|e| Error::Internal(format!("migrate up: {e}")))?,
        MigrateCmd::Down(n) => M::down(&conn, Some(n))
            .await
            .map_err(|e| Error::Internal(format!("migrate down: {e}")))?,
        MigrateCmd::Status => print_status::<M>(&conn).await?,
    }
    Ok(())
}

async fn print_status<M: MigratorTrait>(conn: &DatabaseConnection) -> Result<(), Error> {
    let rows = M::get_migration_with_status(conn)
        .await
        .map_err(|e| Error::Internal(format!("migrate status: {e}")))?;
    let mut applied = 0usize;
    let mut pending = 0usize;
    for m in &rows {
        let status = m.status();
        match status {
            sea_orm_migration::MigrationStatus::Applied => applied += 1,
            sea_orm_migration::MigrationStatus::Pending => pending += 1,
        }
        println!("{status:<9} {}", m.name());
    }
    println!("---");
    println!("{applied} applied, {pending} pending");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn parse_defaults_and_steps() {
        assert_eq!(parse_migrate_args(&s(&[])).unwrap(), MigrateCmd::Up(None));
        assert_eq!(parse_migrate_args(&s(&["up"])).unwrap(), MigrateCmd::Up(None));
        assert_eq!(
            parse_migrate_args(&s(&["up", "3"])).unwrap(),
            MigrateCmd::Up(Some(3))
        );
        assert_eq!(parse_migrate_args(&s(&["down"])).unwrap(), MigrateCmd::Down(1));
        assert_eq!(
            parse_migrate_args(&s(&["down", "2"])).unwrap(),
            MigrateCmd::Down(2)
        );
        assert_eq!(
            parse_migrate_args(&s(&["status"])).unwrap(),
            MigrateCmd::Status
        );
    }

    #[test]
    fn parse_rejects_bad() {
        assert!(parse_migrate_args(&s(&["up", "0"])).is_err());
        assert!(parse_migrate_args(&s(&["up", "x"])).is_err());
        assert!(parse_migrate_args(&s(&["fresh"])).is_err());
        assert!(parse_migrate_args(&s(&["status", "1"])).is_err());
    }
}
