use crate::runner::ContainerRunnerBuilder;
use crate::running::RunningContainer;
use bb8_postgres::{tokio_postgres::NoTls, PostgresConnectionManager};
use bollard::models::HealthConfig;

/// Container with default database
/// database: postgres
/// user: postgres
/// password: pass
/// host-port: 5432
/// container-port: port
pub async fn running_container(
    container_name: &str,
    port: u16,
    container_reg: Option<String>,
) -> anyhow::Result<RunningContainer> {
    let container_name = format!("pg_{}-{}", container_name, port);
    let docker_image = docker_image(container_reg);

    let running_container = ContainerRunnerBuilder::new(container_name)
        .image(docker_image)
        .add_port_binding(port, 5432)
        .add_env_var("POSTGRES_PASSWORD", "pass")
        .healthcheck(HealthConfig {
            test: Some(vec![
                "CMD-SHELL".to_string(),
                "pg_isready -U postgres".to_string(),
            ]),
            interval: Some(250_000_000), // 250ms
            timeout: Some(100_000_000),  // 100ms
            retries: Some(5),
            start_period: Some(500_000_000),
            start_interval: None,
        })
        .build()?
        .run()
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(running_container)
}

pub async fn running_container_with(
    container_name: &str,
    user: &str,
    pass: &str,
    db: &str,
    port: u16,
    container_reg: Option<String>,
) -> anyhow::Result<RunningContainer> {
    let container_name = format!("pg_{}-{}", container_name, port);
    let docker_image = docker_image(container_reg);
    let running_container = ContainerRunnerBuilder::new(container_name)
        .image(docker_image)
        .add_env_var("POSTGRES_USER", user)
        .add_env_var("POSTGRES_PASSWORD", pass)
        .add_env_var("POSTGRES_DB", db)
        .add_port_binding(port, 5432)
        .healthcheck(HealthConfig {
            test: Some(vec![
                "CMD-SHELL".to_string(),
                "pg_isready -U postgres".to_string(),
            ]),
            interval: Some(250_000_000), // 250ms
            timeout: Some(100_000_000),  // 100ms
            retries: Some(5),
            start_period: Some(500_000_000),
            start_interval: None,
        })
        .build()?
        .run()
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(running_container)
}

fn docker_image(container_reg: Option<String>) -> String {
    let cg = container_reg.unwrap_or_else(|| container_registry());
    std::env::var("PG_DOCKER_IMAGE").unwrap_or_else(|_| format!("{}postgres:latest", cg))
}

fn container_registry() -> String {
    std::env::var("PG_CONTAINER_REGISTRY")
        .unwrap_or_else(|_| "public.ecr.aws/docker/library/".to_string())
}

pub async fn pool(
    url: &str,
) -> anyhow::Result<bb8_postgres::bb8::Pool<PostgresConnectionManager<NoTls>>> {
    let manager = bb8_postgres::PostgresConnectionManager::new_from_stringlike(url, NoTls)?;
    let pool = bb8_postgres::bb8::Pool::builder()
        .max_size(30)
        .build(manager)
        .await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use crate::pg::pool;

    async fn test_pool(url: &str) -> anyhow::Result<()> {
        let pool = pool(url).await?;
        let _conn = pool.get().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_pg() -> anyhow::Result<()> {
        let rc = super::running_container("demo-container-1", 5431, None)
            .await
            .expect("cannot create the container");
        println!("container 1 is running");

        test_pool("postgres://postgres:pass@127.0.0.1:5431/postgres").await?;

        rc.stop().await?;
        rc.remove().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_pg_2() -> anyhow::Result<()> {
        let rc =
            super::running_container_with("demo-container-2", "user", "pass", "db", 5432, None)
                .await
                .expect("cannot create the container");
        println!("container 2 is running");

        test_pool("postgres://user:pass@127.0.0.1:5432/db").await?;

        rc.stop().await?;
        rc.remove().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_pg_3() -> anyhow::Result<()> {
        let rc =
            super::running_container_with("demo-container-3", "user2", "pass2", "db2", 5433, None)
                .await
                .expect("cannot create the container");

        println!("container 2 is running");

        test_pool("postgres://user2:pass2@127.0.0.1:5433/db2").await?;

        rc.stop().await?;
        rc.remove().await?;

        Ok(())
    }
}
