use crate::runner::ContainerRunnerBuilder;
use crate::running::RunningContainer;
use bollard::models::HealthConfig;

/// MySQL Container with default db-config
/// database: mysqldb
/// user: root-demo
/// root-password: pass
/// host-port: <port>
/// container-port: 3306
pub async fn running_container(
    container_name: &str,
    port: u16,
    container_reg: Option<String>,
) -> anyhow::Result<RunningContainer> {
    let container_name = format!("mysql_{}-{}", container_name, port);
    let docker_image = docker_image(container_reg);

    let running_container = ContainerRunnerBuilder::new(container_name)
        .image(docker_image)
        .add_port_binding(port, 3306)
        .add_env_var("MYSQL_ROOT_PASSWORD", "pass")
        .add_env_var("MYSQL_DATABASE", "mysqldb")
        .add_env_var("MYSQL_USER", "root-demo")
        .add_env_var("MYSQL_PASSWORD", "pass")
        .healthcheck(HealthConfig {
            test: Some(vec![
                "CMD-SHELL".to_string(),
                "mysqladmin ping --host=127.0.0.1 --port=3306 --password=pass".to_string(),
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
    std::env::var("MYSQL_DOCKER_IMAGE").unwrap_or_else(|_| format!("{}mysql:latest", cg))
}

fn container_registry() -> String {
    std::env::var("MYSQL_CONTAINER_REGISTRY")
        .unwrap_or_else(|_| "public.ecr.aws/docker/library/".to_string())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_mysql() -> anyhow::Result<()> {
        let rc = super::running_container("demo-container-1", 3306, None)
            .await
            .expect("cannot create the container");
        println!("container 1 is running");

        // crate::db::pg::tests::test_pool("postgres://postgres:pass@127.0.0.1:5431/postgres").await?;

        // rc.stop().await?;
        // rc.remove().await?;
        Ok(())
    }
}
